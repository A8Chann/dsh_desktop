'use strict';
const { spawn, execFile } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const http = require('node:http');
const { resolveNode, resolveDsh } = require('./dsh-resolve');

const URL_LINE_RE = /dsh web:\s*(https?:\/\/[^\s]+)/i;
const DSH_SIGNATURE = '__DSH_BOOT__';
const PROFILE_WATCH_FILES = new Set(['package.json', 'pnpm-lock.yaml']);
const RETRY_DELAYS_SEC = [1, 2, 4, 8, 15, 30];
const QUIET_SEC = 6; // 插件变更后等待文件稳定的秒数
const EXTERNAL_HEALTH_MS = 5000; // 外部实例健康检查间隔
const EXTERNAL_FAIL_LIMIT = 3;

function execFileAsync(file, args, timeoutMs) {
  return new Promise((resolve) => {
    execFile(file, args, { timeout: timeoutMs, windowsHide: true }, (err, stdout) => resolve({ err, stdout: String(stdout || '') }));
  });
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

/** 通过 netstat 找监听 127.0.0.1:port 的 PID。 */
async function findPidByPort(port) {
  const { err, stdout } = await execFileAsync('netstat.exe', ['-ano'], 8000);
  if (err) return null;
  const wanted = `127.0.0.1:${port}`;
  for (const line of stdout.split(/\r?\n/)) {
    const parts = line.trim().split(/\s+/);
    if (parts.length >= 5 && parts[0] === 'TCP' && parts[1] === wanted && parts[3] === 'LISTENING') {
      const pid = Number(parts[4]);
      if (Number.isFinite(pid) && pid > 0) return pid;
    }
  }
  return null;
}

/**
 * dsh 后端进程管理器：
 *  - 以子进程拉起 `dsh web`（自动解析 node/dsh 入口），解析 stdout 的
 *    `dsh web: http://127.0.0.1:<port>` 行得到就绪 URL；
 *  - 若目标端口已被一个 dsh web 实例占用（探测 __DSH_BOOT__ 签名），
 *    则「接管复用」该外部实例；外部实例消失后自动接管拉起自己的；
 *  - 端口被非 dsh 服务占用时回退 --port 0（系统分配）；
 *  - 监控 profile 目录的 package.json / pnpm-lock.yaml，插件变更稳定后
 *    触发可取消的重启流程；
 *  - 后端意外退出按退避策略自动重启。
 */
class Backend {
  constructor({ settings, logger, events }) {
    this.settings = settings;
    this.log = logger;
    this.events = events; // { onStatus(status), onPluginChange() }
    this.child = null;
    this.externalPid = null;
    this.url = null;
    this.port = null;
    this.state = 'idle'; // idle|starting|running|external|restart-pending|restarting|error|stopped
    this.error = null;
    this.nextRetryMs = 0;
    this.retryCount = 0;
    this.installState = null; // npx 自动安装进度 {phase, detail, fetched}
    this.stopping = false;
    this.quietTimer = null;
    this.retryTimer = null;
    this.healthTimer = null;
    this.externalFailStreak = 0;
    this.watcher = null;
    this.mock = process.env.DSH_DESKTOP_MOCK === '1';
    this.dshHome = process.env.DSH_HOME || path.join(os.homedir(), '.dsh');
    this.profileDir =
      process.env.DSH_DESKTOP_PROFILE || path.join(this.dshHome, 'profiles', settings.profile || 'web');
    this.nodeBin = settings.nodeBin || resolveNode();
  }

  emitStatus() {
    const pid = this.child ? this.child.pid : this.externalPid || null;
    this.events.onStatus({
      state: this.state,
      url: this.url,
      port: this.port,
      pid,
      owned: !this.externalPid,
      error: this.error,
      nextRetrySec: this.nextRetryMs ? Math.round(this.nextRetryMs / 1000) : 0,
      profileDir: this.profileDir,
      mock: this.mock,
      install: this.installState || null
    });
  }

  // ─────────────────────────── 启动 ───────────────────────────

  async start() {
    if (this.stopping) return;
    this.state = 'starting';
    this.error = null;
    this.emitStatus();

    // mock 模式直接拉起 mock 后端，不做端口探测
    if (this.mock) {
      await this.spawnOwn(this.settings.port || 0);
      return;
    }

    const port = this.settings.port || 0;
    if (port !== 0) {
      const probe = await this.probePort(port);
      if (probe.ok && probe.dsh) {
        this.adoptExternal(port, probe.pid);
        return;
      }
      if (probe.ok) {
        this.log.warn(`端口 ${port} 被非 dsh 服务占用，回退到系统分配端口`);
        await this.spawnOwn(0);
        return;
      }
    }
    await this.spawnOwn(port);
  }

  /** 接管一个已在运行的外部 dsh web 实例。 */
  adoptExternal(port, pid) {
    this.externalPid = pid;
    this.url = `http://127.0.0.1:${port}`;
    this.port = port;
    this.state = 'external';
    this.error = null;
    this.retryCount = 0;
    this.log.info(`复用外部 dsh web 实例: ${this.url} (pid ${pid})`);
    this.emitStatus();
    this.startHealthWatch();
  }

  /** 真正拉起（或回退拉起）自己的 dsh web 子进程。 */
  async spawnOwn(port) {
    this.externalPid = null;
    this.stopHealthWatch();
    this.port = port;
    this.state = 'starting';
    this.emitStatus();

    let cmd;
    let cmdArgs;
    if (this.mock) {
      cmd = this.nodeBin;
      cmdArgs = [this.mockBackendPath()];
      this.log.info(`[mock] 启动 mock 后端: ${cmd} ${cmdArgs.join(' ')}`);
    } else {
      const dsh = resolveDsh(this.settings.dshBin);
      if (!dsh) {
        // 未找到本地 dsh：自动通过 npx 下载启动（npx 自动安装到 %LOCALAPPDATA%\npm-cache\_npx，
        // 下次启动即被 resolveDsh 复用缓存，无需再次下载）
        cmd = 'cmd.exe';
        cmdArgs = ['/d', '/s', '/c', `npx -y @deepseek-ai/dsh web --port ${port}`];
        this.log.info('未找到本地 dsh 安装，将通过 npx 自动下载启动（首次需联网）');
      } else if (dsh.kind === 'node') {
        cmd = this.nodeBin;
        cmdArgs = [dsh.bin, 'web', '--port', String(port)];
      } else {
        cmd = 'cmd.exe';
        cmdArgs = ['/d', '/s', '/c', `""${dsh.shim}" web --port ${port}"`];
      }
      this.log.info(`启动 dsh: ${cmd} ${cmdArgs.join(' ')} (${dsh.source})`);
    }

    let child;
    try {
      // npx 自动安装分支：让 npm 输出 info 级进度（默认管道下无进度，用户看不到）
      const env = { ...process.env };
      if (cmd === 'cmd.exe' && cmdArgs.join(' ').includes('npx -y')) {
        env.npm_config_loglevel = 'info';
        env.npm_config_color = 'false';
        env.npm_config_fund = 'false';
        this.installState = { phase: 'downloading', detail: '正在连接 npm 镜像…', fetched: 0 };
      }
      child = spawn(cmd, cmdArgs, {
        cwd: this.settings.workspace,
        env,
        windowsHide: true,
        stdio: ['ignore', 'pipe', 'pipe']
      });
    } catch (e) {
      this.state = 'error';
      this.error = `无法启动 dsh 进程: ${e.message}`;
      this.log.error(this.error);
      this.emitStatus();
      this.scheduleRetry();
      return;
    }
    this.child = child;

    let buffer = '';
    let urlFound = false;
    let stderrTail = '';

    // npx 安装进度：从 info 级输出提取「已获取 N 个包 / 最近包名 / 完成」
    const trackInstall = (line) => {
      if (!this.installState) return;
      if (/npm http fetch GET/.test(line)) {
        this.installState.fetched += 1;
        const m = line.match(/https:\/\/[^/]+\/([^/?#]+)/);
        this.installState.detail = `正在下载依赖包（已获取 ${this.installState.fetched} 个）${m ? '：' + m[1] : ''}`;
      } else if (/added \d+ packages/.test(line)) {
        this.installState.phase = 'finishing';
        this.installState.detail = '下载完成，正在启动 dsh…';
      } else if (/npm error/i.test(line)) {
        this.installState.phase = 'error';
        this.installState.detail = line.replace(/^.*npm error\s*/i, '').slice(0, 120);
      }
      this.emitStatus(); // 推送给悬浮条/状态页
    };

    child.stdout.on('data', (chunk) => {
      buffer += chunk.toString();
      let idx;
      while ((idx = buffer.indexOf('\n')) >= 0) {
        const line = buffer.slice(0, idx).trim();
        buffer = buffer.slice(idx + 1);
        if (!line) continue;
        this.log.write('dsh', line);
        trackInstall(line);
        if (!urlFound) {
          const m = line.match(URL_LINE_RE);
          if (m) {
            urlFound = true;
            this.onUrl(m[1]);
          }
        }
      }
    });
    child.stderr.on('data', (chunk) => {
      const text = chunk.toString();
      stderrTail = (stderrTail + text).slice(-2000);
      this.log.write('dsh-err', text.trim());
      // npm 的 info/warn 大多走 stderr
      for (const line of text.split(/\r?\n/)) {
        const t = line.trim();
        if (t) trackInstall(t);
      }
    });
    child.on('error', (err) => {
      this.log.error(`dsh 进程错误: ${err.message}`);
      stderrTail += `\n${err.message}`;
    });
    child.on('exit', (code, signal) => {
      if (this.child !== child) return;
      this.child = null;
      this.onChildExit(code, signal, { urlFound, stderrTail, startedPort: port });
    });

    // 启动超 60s 仍未就绪时给出提示（不中断，继续等待）
    setTimeout(() => {
      if (!urlFound && this.child === child) {
        this.log.warn('dsh 启动超过 60s 仍未输出 URL，继续等待…');
      }
    }, 60000);
  }

  /** 打包后代码位于 app.asar 内，外部 node.exe 无法读取 asar；把 mock 脚本复制到临时目录再启动。 */
  mockBackendPath() {
    const dest = path.join(os.tmpdir(), 'dsh-desktop-mock-backend.js');
    try {
      fs.copyFileSync(path.join(__dirname, 'mock-backend.js'), dest);
    } catch (e) {
      this.log.warn(`mock 脚本复制失败: ${e.message}`);
    }
    return dest;
  }

  onUrl(url) {
    this.url = url;
    try {
      this.port = Number(new URL(url).port);
    } catch {}
    this.retryCount = 0;
    this.nextRetryMs = 0;
    this.installState = null; // 就绪后清除安装进度
    this.state = 'running';
    this.error = null;
    this.log.info(`dsh web 就绪: ${url} (pid ${this.child ? this.child.pid : '?'})`);
    this.emitStatus();
  }

  onChildExit(code, signal, info) {
    if (this.stopping) {
      this.state = 'stopped';
      this.emitStatus();
      return;
    }
    const tail = (info.stderrTail || '').trim();
    if (info.urlFound) {
      // 曾经就绪过：意外退出 → 退避重启
      this.log.warn(`dsh 后端意外退出 (code=${code} signal=${signal})，安排自动重启`);
      this.state = 'error';
      this.error = `后端进程退出 (code=${code}${signal ? `, ${signal}` : ''})`;
      this.emitStatus();
      this.scheduleRetry();
      return;
    }
    if (/EADDRINUSE|address already in use|address in use/i.test(tail) && info.startedPort !== 0) {
      this.log.warn(`端口 ${info.startedPort} 绑定失败，检查是否已有 dsh 实例`);
      this.probePort(info.startedPort).then((probe) => {
        if (probe.ok && probe.dsh) {
          this.adoptExternal(info.startedPort, probe.pid);
        } else {
          this.spawnOwn(0);
        }
      });
      return;
    }
    this.state = 'error';
    this.error = `dsh 启动失败 (code=${code})${tail ? `: ${tail.split('\n').slice(-3).join(' ')}` : ''}`;
    this.log.error(this.error);
    this.emitStatus();
    this.scheduleRetry();
  }

  scheduleRetry() {
    if (this.stopping) return;
    const idx = Math.min(this.retryCount, RETRY_DELAYS_SEC.length - 1);
    this.retryCount += 1;
    this.nextRetryMs = RETRY_DELAYS_SEC[idx] * 1000;
    this.log.warn(`将在 ${this.nextRetryMs / 1000}s 后重试`);
    this.emitStatus();
    clearTimeout(this.retryTimer);
    this.retryTimer = setTimeout(() => {
      if (this.stopping) return;
      this.start();
    }, this.nextRetryMs);
  }

  // ─────────────────────────── 重启 ───────────────────────────

  /** 供主进程调用的重启（手动 / 插件变更倒计时结束）。 */
  async restart(reason) {
    if (this.stopping) return;
    this.log.info(`==== 重启后端: ${reason} ====`);
    this.state = 'restarting';
    this.error = null;
    clearTimeout(this.retryTimer);
    this.nextRetryMs = 0;
    this.emitStatus();
    await this.killCurrent();
    await this.start();
  }

  async killCurrent() {
    const tasks = [];
    if (this.child) {
      const pid = this.child.pid;
      try {
        this.child.kill();
      } catch {}
      this.child = null;
      tasks.push(sleep(1200));
      tasks.push(this.killTree(pid));
    }
    if (this.externalPid) {
      const pid = this.externalPid;
      this.externalPid = null;
      tasks.push(this.killTree(pid));
    }
    await Promise.all(tasks);
  }

  killTree(pid) {
    return execFileAsync('taskkill.exe', ['/PID', String(pid), '/T', '/F'], 6000);
  }

  // ─────────────────────────── 外部实例健康检查 ───────────────────────────

  startHealthWatch() {
    this.stopHealthWatch();
    this.externalFailStreak = 0;
    this.healthTimer = setInterval(() => {
      if (this.stopping || this.state !== 'external' || !this.url) return;
      this.probeUrl(this.url).then((ok) => {
        if (this.stopping || this.state !== 'external') return;
        if (ok) {
          this.externalFailStreak = 0;
        } else {
          this.externalFailStreak += 1;
          this.log.warn(`外部实例健康检查失败 ${this.externalFailStreak}/${EXTERNAL_FAIL_LIMIT}`);
          if (this.externalFailStreak >= EXTERNAL_FAIL_LIMIT) {
            this.log.warn('外部 dsh 实例已消失，接管启动自有后端');
            this.takeOver();
          }
        }
      });
    }, EXTERNAL_HEALTH_MS);
  }

  stopHealthWatch() {
    clearInterval(this.healthTimer);
    this.healthTimer = null;
    this.externalFailStreak = 0;
  }

  async takeOver() {
    if (this.stopping) return;
    await this.killCurrent();
    await this.start();
  }

  // ─────────────────────────── 探测 ───────────────────────────

  probeUrl(url) {
    return new Promise((resolve) => {
      const req = http.get(url, { timeout: 3000 }, (res) => {
        res.resume();
        res.on('end', () => resolve(res.statusCode === 200));
        res.on('error', () => resolve(false));
      });
      req.on('timeout', () => {
        req.destroy();
        resolve(false);
      });
      req.on('error', () => resolve(false));
    });
  }

  /** 探测端口：{ ok, dsh, pid } —— ok=有服务响应；dsh=响应含 dsh 前端签名。 */
  async probePort(port) {
    const result = { ok: false, dsh: false, pid: null };
    try {
      await new Promise((resolve) => {
        const req = http.get(`http://127.0.0.1:${port}/`, { timeout: 2500 }, (res) => {
          let body = '';
          res.setEncoding('utf8');
          res.on('data', (c) => {
            body += c;
            if (body.length > 200000) req.destroy();
          });
          res.on('end', () => {
            result.ok = res.statusCode === 200;
            result.dsh = body.includes(DSH_SIGNATURE);
            resolve();
          });
          res.on('error', () => resolve());
        });
        req.on('timeout', () => {
          req.destroy();
          resolve();
        });
        req.on('error', () => resolve());
      });
    } catch {}
    if (result.ok) {
      result.pid = await findPidByPort(port);
    }
    return result;
  }

  // ─────────────────────────── 插件变更监控 ───────────────────────────

  startProfileWatch() {
    this.stopProfileWatch();
    if (!fs.existsSync(this.profileDir)) {
      this.log.warn(`profile 目录不存在，插件变更监控已禁用: ${this.profileDir}`);
      return;
    }
    try {
      this.watcher = fs.watch(this.profileDir, { persistent: true }, (evt, name) => {
        if (name && PROFILE_WATCH_FILES.has(name)) {
          this.log.info(`profile 文件变化: ${name} (${evt})`);
          this.armQuietTimer();
        }
      });
      this.log.info(`插件变更监控已启动: ${this.profileDir}`);
    } catch (e) {
      this.log.warn(`插件变更监控启动失败: ${e.message}`);
    }
  }

  stopProfileWatch() {
    if (this.watcher) {
      try {
        this.watcher.close();
      } catch {}
      this.watcher = null;
    }
    clearTimeout(this.quietTimer);
    this.quietTimer = null;
  }

  armQuietTimer() {
    clearTimeout(this.quietTimer);
    this.quietTimer = setTimeout(() => this.onQuietPeriod(), QUIET_SEC * 1000);
  }

  async onQuietPeriod() {
    if (this.stopping) return;
    if (await this.isPnpmRunning()) {
      this.log.info('pnpm 仍在运行（安装尚未结束），继续等待…');
      this.armQuietTimer();
      return;
    }
    this.log.info('插件变更已稳定，通知主进程');
    this.events.onPluginChange();
  }

  isPnpmRunning() {
    return new Promise((resolve) => {
      // 插件安装期间的存活进程：
      //  1. pnpm 本体（pnpm.cmd → node corepack\dist\pnpm.js，或独立 pnpm.exe）；
      //  2. dsh plugin 转发进程（node <dsh>\lib\bin.js plugin ...，pnpm 是其同步
      //     子进程，pnpm 结束它才退出）——文件变化常发生在安装末尾，仅查 pnpm
      //     可能误判，把 dsh plugin 进程一并纳入。
      // 排除查询进程自身，避免命令行里含 'pnpm' 字样导致自匹配误报。
      const ps =
        "Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -ne $PID -and $_.CommandLine -and (($_.Name -eq 'pnpm.exe') -or ($_.Name -eq 'node.exe' -and ($_.CommandLine -match 'pnpm' -or $_.CommandLine -match 'bin\\.js.{0,40}plugin'))) } | Measure-Object | Select-Object -ExpandProperty Count";
      execFile(
        'powershell.exe',
        ['-NoProfile', '-NonInteractive', '-Command', ps],
        { timeout: 10000, windowsHide: true },
        (err, stdout) => {
          if (err) {
            resolve(false);
            return;
          }
          const n = parseInt(String(stdout).trim(), 10);
          resolve(Number.isFinite(n) && n > 0);
        }
      );
    });
  }

  // ─────────────────────────── 停止 ───────────────────────────

  async stop() {
    if (this.stopping) return;
    this.stopping = true;
    clearTimeout(this.quietTimer);
    clearTimeout(this.retryTimer);
    this.stopHealthWatch();
    this.stopProfileWatch();
    await this.killCurrent();
    this.state = 'stopped';
    this.emitStatus();
    this.log.info('后端已停止');
  }
}

module.exports = { Backend };
