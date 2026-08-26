'use strict';
const { app, BrowserWindow, Menu, Tray, shell, dialog, ipcMain, Notification, session, nativeImage } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const { spawn, execFile } = require('node:child_process');
const { Logger } = require('./lib/logger');
const { loadSettings, saveSettings, settingsFile } = require('./lib/settings');
const { Backend } = require('./lib/backend');
const { resolveNode, resolveDsh } = require('./lib/dsh-resolve');

const APP_NAME = 'DSH Desktop';
const STATUS_PAGE = path.join(__dirname, 'lib', 'status-page.html');

let settings;
let logger;
let backend;
let win = null;
let tray = null;
let quitting = false;
let countdownTimer = null;
let restartPending = false;

const userDataDir = () => app.getPath('userData');
const log = (...args) => logger && logger.info(args.join(' '));

// ─────────────────────────── 单实例锁 ───────────────────────────
const gotLock = app.requestSingleInstanceLock();
if (!gotLock) {
  app.quit();
} else {
  app.on('second-instance', () => {
    if (win) {
      if (win.isMinimized()) win.restore();
      win.show();
      win.focus();
    }
  });
}

// ─────────────────────────── 后端状态编排 ───────────────────────────

function sendStatus() {
  if (!win || win.isDestroyed()) return;
  win.webContents.send('backend-status', lastStatus);
}
let lastStatus = null;
let needReload = false; // 后端重启完成后需要刷新前端页面

function onStatus(status) {
  const prev = lastStatus;
  lastStatus = status;
  // 后端从「重启/启动中」变为「就绪」时，前端页面在重启后不会自动刷新（URL 没变），
  // 标记一次强制刷新，让 GUI 与重启后的后端重新建立连接。
  if (status && (status.state === 'running' || status.state === 'external') && status.url) {
    if (prev && (prev.state === 'restarting' || prev.state === 'starting' || prev.state === 'restart-pending')) {
      needReload = true;
    }
    scheduleAutoReport();
  }
  sendStatus();
  syncWindow();
  updateTrayTooltip();
}

// ─────────────────────────── 重启后自动汇报 ───────────────────────────
// 主进程代装完成 → 后端重启 → 会话恢复后，向最新活跃会话自动发一条消息，
// 让 agent 回合自动醒来读取 result-*.json 并汇报，用户无需任何操作。
const reportedResults = new Set();
let autoReportTimer = null;

function scheduleAutoReport() {
  clearTimeout(autoReportTimer);
  autoReportTimer = setTimeout(() => {
    autoReportAfterRestart().catch((e) => log(`[auto-report] 失败: ${e.message}`));
  }, 3000); // 等会话持久化恢复
}

function apiPost(base, method, payload) {
  return fetch(`${base}/api/${method}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ type: 'client-request', rpcId: `dshd-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`, method, payload })
  }).then(async (res) => {
    if (res.status !== 200) throw new Error(`${method} -> HTTP ${res.status}`);
    return res.json();
  });
}

async function findLatestSession(base) {
  const r = await apiPost(base, 'session.list', {});
  if (!r || r.result?.ok !== true) throw new Error('session.list 失败');
  const items = (r.result.value && r.result.value.items) || [];
  if (items.length === 0) return null;
  items.sort((a, b) => b.updatedAt - a.updatedAt);
  return items[0].sessionId;
}

async function autoReportAfterRestart() {
  if (!lastStatus || !lastStatus.url) return;
  const dir = controlDir();
  let files = [];
  try {
    files = fs.readdirSync(dir).filter((f) => f.startsWith('result-') && f.endsWith('.json'));
  } catch {
    return;
  }
  // 持久化去重：result 文件里写入 reportedAt 标记，跨实例重启也只汇报一次
  const pending = [];
  for (const f of files) {
    if (reportedResults.has(f)) continue;
    let result = null;
    try {
      result = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8').replace(/^\uFEFF/, ''));
    } catch {}
    if (result && result.reportedAt) continue;
    pending.push(f);
  }
  if (pending.length === 0) return;
  const sessionId = await findLatestSession(lastStatus.url);
  if (!sessionId) return;
  for (const f of pending) {
    reportedResults.add(f);
    const resultPath = path.join(dir, f);
    const text = `【系统】插件安装已完成，请读取 ${resultPath} 中的安装结果，并向用户简洁汇报（汇报后删除该 result 文件）。`;
    try {
      await apiPost(lastStatus.url, 'session.prompt', { sessionId, mode: 'queue', content: [{ type: 'text', text }] });
      log(`[auto-report] 已向 ${sessionId} 发送汇报提示: ${f}`);
      // 写回 reportedAt 标记（agent 汇报后删除文件，此标记兜底防重复）
      try {
        const current = fs.existsSync(resultPath) ? fs.readFileSync(resultPath, 'utf8') : null;
        if (current) {
          const parsed = JSON.parse(current.replace(/^\uFEFF/, ''));
          parsed.reportedAt = new Date().toISOString();
          fs.writeFileSync(resultPath, JSON.stringify(parsed, null, 2));
        }
      } catch {}
    } catch (e) {
      log(`[auto-report] 发送失败 ${f}: ${e.message}`);
    }
  }
}

function notify(title, body) {
  if (!Notification.isSupported()) return;
  try {
    const n = new Notification({ title: `${APP_NAME} — ${title}`, body, silent: true });
    n.on('click', () => {
      if (win) {
        win.show();
        win.focus();
      }
    });
    n.show();
  } catch {}
}

/** 插件变更 → 仅提示，不再自动重启（用户手动重启后端以加载新插件）。 */
function onPluginChange() {
  log('插件已变更：请手动重启后端以加载新插件');
  notify('插件已变更', '请手动重启后端（悬浮条「重启」或 Ctrl+Shift+R）以加载新插件');
}

function sendCountdown(seconds) {
  if (!win || win.isDestroyed()) return;
  win.webContents.send('backend-countdown', { seconds, reason: 'plugin-change' });
}

function cancelRestart() {
  if (countdownTimer) {
    clearInterval(countdownTimer);
    countdownTimer = null;
  }
  if (restartPending) {
    restartPending = false;
    log('自动重启已取消');
    if (lastStatus) sendStatus();
  }
}

/** 根据后端状态决定窗口显示什么。 */
function syncWindow() {
  if (!win || win.isDestroyed() || !lastStatus) return;
  const current = win.webContents.getURL();
  const s = lastStatus;

  if (s.url && (s.state === 'running' || s.state === 'external')) {
    if (!current.startsWith(s.url)) {
      needReload = false;
      win.loadURL(s.url);
    } else if (needReload) {
      needReload = false;
      log('后端已重启，刷新前端页面');
      win.webContents.reloadIgnoringCache();
    }
    return;
  }
  if (s.url) {
    // starting / restarting / restart-pending：如果当前不在后端页，则不动；若在则保持
    if (current.startsWith('chrome-error://') || current.startsWith('file://')) {
      win.loadURL(s.url);
    }
    return;
  }
  // 完全没有 URL：显示本地状态页
  if (!current.startsWith('file://')) {
    win.loadFile(STATUS_PAGE);
  }
}

// ─────────────────────────── 窗口 ───────────────────────────

function createWindow() {
  win = new BrowserWindow({
    width: 1440,
    height: 900,
    minWidth: 960,
    minHeight: 640,
    title: APP_NAME,
    icon: path.join(__dirname, 'assets', 'icon.png'),
    backgroundColor: '#0b1220',
    autoHideMenuBar: true,
    show: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      spellcheck: false
    }
  });

  win.once('ready-to-show', () => win.show());

  win.on('close', (e) => {
    if (settings.closeToTray && !quitting) {
      e.preventDefault();
      win.hide();
    }
  });

  win.on('closed', () => {
    win = null;
  });

  // 外部链接交给系统浏览器
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url && (url.startsWith('http://') || url.startsWith('https://'))) shell.openExternal(url);
    return { action: 'deny' };
  });
  win.webContents.on('will-navigate', (e, url) => {
    const ok = url.startsWith('http://127.0.0.1:') || url.startsWith('http://localhost:') || url.startsWith('file://');
    if (!ok) {
      e.preventDefault();
      if (url.startsWith('http://') || url.startsWith('https://')) shell.openExternal(url);
    }
  });
}

// ─────────────────────────── 菜单 / 托盘 ───────────────────────────

function openInBrowser() {
  if (lastStatus && lastStatus.url) shell.openExternal(lastStatus.url);
}

function openLogsDir() {
  shell.openPath(path.join(userDataDir(), 'logs'));
}

function buildMenu() {
  const template = [
    {
      label: '应用',
      submenu: [
        {
          label: '重启后端',
          accelerator: 'CmdOrCtrl+Shift+R',
          click: () => backend.restart('manual')
        },
        {
          label: '在浏览器打开',
          accelerator: 'CmdOrCtrl+Shift+O',
          click: openInBrowser
        },
        { type: 'separator' },
        { label: '打开日志目录', click: openLogsDir },
        { type: 'separator' },
        { label: '退出', accelerator: 'Alt+F4', click: () => app.quit() }
      ]
    },
    {
      label: '视图',
      submenu: [
        { label: '重新加载页面', role: 'reload' },
        { label: '开发者工具', role: 'toggleDevTools' },
        { type: 'separator' },
        { label: '实际大小', role: 'resetZoom' },
        { label: '放大', role: 'zoomIn' },
        { label: '缩小', role: 'zoomOut' },
        { type: 'separator' },
        { label: '全屏', role: 'togglefullscreen' }
      ]
    },
    {
      label: '帮助',
      submenu: [
        {
          label: `关于 ${APP_NAME}`,
          click: () => {
            dialog.showMessageBox(win, {
              type: 'info',
              title: `关于 ${APP_NAME}`,
              message: `${APP_NAME} v${app.getVersion()}`,
              detail:
                `DeepSeek Harness 桌面端\n\n` +
                `后端状态: ${lastStatus ? lastStatus.state : 'unknown'}\n` +
                `URL: ${lastStatus && lastStatus.url ? lastStatus.url : '—'}\n` +
                `profile: ${settings.profile} (${lastStatus ? lastStatus.profileDir : '?'})\n\n` +
                `日志: ${path.join(userDataDir(), 'logs', 'main.log')}\n` +
                `设置: ${path.join(userDataDir(), 'settings.json')}`
            });
          }
        }
      ]
    }
  ];
  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}

function updateTrayTooltip() {
  if (!tray) return;
  const s = lastStatus;
  const stateText = s
    ? { running: '运行中', external: '运行中(外部)', starting: '启动中', restarting: '重启中', error: '异常', stopped: '已停止' }[s.state] || s.state
    : '未就绪';
  tray.setToolTip(`${APP_NAME} — 后端${stateText}${s && s.port ? ` :${s.port}` : ''}`);
}

function createTray() {
  const iconPath = path.join(__dirname, 'assets', 'icon.png');
  let image;
  try {
    image = nativeImage.createFromPath(iconPath).resize({ width: 16, height: 16 });
  } catch {
    return;
  }
  tray = new Tray(image);
  const menu = Menu.buildFromTemplate([
    { label: '显示主窗口', click: () => { win.show(); win.focus(); } },
    { label: '重启后端', click: () => backend.restart('manual') },
    { label: '在浏览器打开', click: openInBrowser },
    { label: '打开日志目录', click: openLogsDir },
    { type: 'separator' },
    { label: '退出', click: () => app.quit() }
  ]);
  tray.setContextMenu(menu);
  tray.on('click', () => {
    if (win) {
      win.show();
      win.focus();
    }
  });
  updateTrayTooltip();
}

// ─────────────────────────── 测试控制通道 ───────────────────────────
// DSH_DESKTOP_TEST_CONTROL=<dir>：轮询目录下的 JSON 命令文件并执行：
//   {"cmd":"screenshot","path":"..."}  {"cmd":"status","path":"..."}
//   {"cmd":"restart"}  {"cmd":"cancel"}  {"cmd":"quit"}
function setupTestControl() {
  const dir = process.env.DSH_DESKTOP_TEST_CONTROL;
  if (!dir) return;
  try {
    fs.mkdirSync(dir, { recursive: true });
  } catch {}
  log(`[test] 控制通道已启用: ${dir}`);
  const timer = setInterval(() => {
    let files = [];
    try {
      files = fs.readdirSync(dir).filter((f) => f.endsWith('.json'));
    } catch {
      return;
    }
    for (const f of files) {
      const file = path.join(dir, f);
      let cmd;
      try {
        cmd = JSON.parse(fs.readFileSync(file, 'utf8'));
      } catch {
        continue;
      } finally {
        try {
          fs.unlinkSync(file);
        } catch {}
      }
      handleTestCommand(cmd).catch((e) => log(`[test] 命令失败: ${JSON.stringify(cmd)} ${e.message}`));
    }
  }, 400);
  app.on('before-quit', () => clearInterval(timer));
}

async function handleTestCommand(cmd) {
  log(`[test] 执行命令: ${JSON.stringify(cmd)}`);
  if (cmd.cmd === 'screenshot') {
    if (!win || win.isDestroyed()) return;
    const image = await win.webContents.capturePage();
    fs.writeFileSync(cmd.path, image.toPNG());
    log(`[test] 截图已保存: ${cmd.path}`);
  } else if (cmd.cmd === 'status') {
    fs.writeFileSync(cmd.path, JSON.stringify(lastStatus || { state: 'none' }, null, 2));
    log(`[test] 状态已保存: ${cmd.path}`);
  } else if (cmd.cmd === 'restart') {
    backend.restart('test');
  } else if (cmd.cmd === 'cancel') {
    cancelRestart();
  } else if (cmd.cmd === 'quit') {
    app.quit();
  }
}

// ─────────────────────────── 插件安装控制通道 ───────────────────────────
// agent（dsh web 会话）在会话内直接跑 `dsh plugin add` 时，安装完成后后端会
// 自动重启，把 agent 正在进行的回合掐断。本通道让 agent 把安装请求**交给主进程
// 执行**：主进程不随后端重启，安装完整结束后由现有插件变更流程弹倒计时重启，
// 结果写回控制目录，agent 重启恢复会话后读取并汇报。
//
// 用法（agent 侧）：
//   1. 写控制目录 cmd-<id>.json：{"cmd":"install-plugin","spec":"<包名或 tarball URL>","id":"<唯一id>","profile":"web","timeoutMs":300000}
//   2. 轮询 result-<id>.json，直到 state ∈ {done, failed, timeout}；
//      或直接结束回合，重启后读该文件汇报。
// 控制目录默认 %APPDATA%\DSH Desktop\control，可用 DSH_DESKTOP_CONTROL_DIR 覆盖。
const controlDir = () => process.env.DSH_DESKTOP_CONTROL_DIR || path.join(userDataDir(), 'control');

function setupPluginInstallChannel() {
  const dir = controlDir();
  try {
    fs.mkdirSync(dir, { recursive: true });
  } catch {}
  log(`插件安装控制通道已启用: ${dir}`);
  const timer = setInterval(() => {
    let files = [];
    try {
      files = fs.readdirSync(dir).filter((f) => f.startsWith('cmd-') && f.endsWith('.json'));
    } catch {
      return;
    }
    for (const f of files) {
      const file = path.join(dir, f);
      let cmd;
      try {
        cmd = JSON.parse(fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '')); // 兼容 UTF-8 BOM
      } catch {
        try {
          fs.unlinkSync(file);
        } catch {}
        continue;
      }
      try {
        fs.unlinkSync(file); // 先删命令文件，保证只被一个实例处理
      } catch {}
      handleInstallCommand(cmd).catch((e) => log(`[install] 命令处理失败: ${e.message}`));
    }
  }, 400);
  app.on('before-quit', () => clearInterval(timer));
}

function writeInstallResult(id, result) {
  const out = path.join(controlDir(), `result-${id}.json`);
  try {
    fs.writeFileSync(out, JSON.stringify(result, null, 2));
    log(`[install] 结果已写入: ${out}`);
  } catch (e) {
    log(`[install] 写结果失败: ${e.message}`);
  }
}

async function handleInstallCommand(cmd) {
  if (!cmd) return;
  // 取消挂起的插件变更自动重启（agent 需要先输出完再重启时使用）
  if (cmd.cmd === 'cancel-restart') {
    log('[install] 收到取消重启请求');
    cancelRestart();
    return;
  }
  // agent 主动触发后端重启（安装完成后、输出完整后使用）
  if (cmd.cmd === 'restart') {
    log('[install] 收到手动重启请求');
    cancelRestart();
    backend.restart('control');
    return;
  }
  if (cmd.cmd !== 'install-plugin') return;
  const id = String(cmd.id || `inst-${Date.now()}`);
  const spec = String(cmd.spec || '').trim();
  const profile = String(cmd.profile || (settings && settings.profile) || 'web').trim();
  if (!spec) {
    writeInstallResult(id, { id, state: 'failed', error: 'spec 为空' });
    return;
  }

  log(`[install] 开始安装: ${spec} (profile=${profile}, id=${id})`);
  writeInstallResult(id, { id, profile, spec, state: 'installing', startedAt: new Date().toISOString() });

  const dsh = resolveDsh(settings && settings.dshBin);
  if (!dsh) {
    writeInstallResult(id, {
      id, profile, spec, state: 'failed',
      error: '未找到 dsh 安装：请重启后端让应用自动通过 npm 安装（选最快源），或在 settings.json 中指定 dshBin'
    });
    return;
  }
  const nodeBin = settings && settings.nodeBin ? settings.nodeBin : resolveNode();
  const profileDir = path.join(process.env.DSH_HOME || path.join(os.homedir(), '.dsh'), 'profiles', profile);
  try {
    fs.mkdirSync(profileDir, { recursive: true }); // spawn 的 cwd 必须存在（dsh plugin 会自行初始化 profile）
  } catch (e) {
    writeInstallResult(id, { id, profile, spec, state: 'failed', error: `无法创建 profile 目录: ${e.message}` });
    return;
  }

  const cmdArgs =
    dsh.kind === 'node'
      ? [dsh.bin, 'plugin', '--profile', profile, 'add', spec]
      : ['/d', '/s', '/c', `""${dsh.shim}" plugin --profile ${profile} add ${spec}"`];

  log(`[install] 执行: ${nodeBin} ${cmdArgs.join(' ')}`);
  const child = spawn(nodeBin, cmdArgs, {
    cwd: profileDir,
    env: process.env,
    windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe']
  });

  let out = '';
  let err = '';
  const cap = 64000;
  child.stdout.on('data', (c) => {
    out = (out + c).slice(-cap);
  });
  child.stderr.on('data', (c) => {
    err = (err + c).slice(-cap);
  });

  const timeoutMs = Number(cmd.timeoutMs) || 300000;
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    log(`[install] 安装超时 (${timeoutMs}ms)，终止进程树`);
    try {
      child.kill();
    } catch {}
    execFile('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], { timeout: 6000, windowsHide: true }, () => {});
  }, timeoutMs);

  child.on('error', (e) => {
    clearTimeout(timer);
    writeInstallResult(id, { id, profile, spec, state: 'failed', error: `启动失败: ${e.message}` });
  });
  child.on('exit', (code, signal) => {
    clearTimeout(timer);
    const state = timedOut ? 'timeout' : code === 0 ? 'done' : 'failed';
    log(`[install] 安装结束: state=${state} code=${code}${signal ? ` signal=${signal}` : ''}`);
    writeInstallResult(id, {
      id,
      profile,
      spec,
      state,
      exitCode: code,
      signal: signal || null,
      output: out.slice(-8000),
      stderr: err.slice(-8000),
      finishedAt: new Date().toISOString()
    });
  });
}

// ─────────────────────────── 生命周期 ───────────────────────────

app.setAppUserModelId('io.dsh.desktop');

app.whenReady().then(() => {
  logger = new Logger(path.join(userDataDir(), 'logs'));
  settings = loadSettings(userDataDir());
  if (!fs.existsSync(settingsFile(userDataDir()))) {
    saveSettings(userDataDir(), settings); // 首次运行生成默认设置文件
  }
  log(`==== ${APP_NAME} v${app.getVersion()} 启动 ====`);
  log(`settings: ${JSON.stringify(settings)}`);
  log(`DSH_HOME=${process.env.DSH_HOME || path.join(os.homedir(), '.dsh')}`);

  backend = new Backend({
    settings,
    logger,
    events: { onStatus, onPluginChange },
    userDataDir: userDataDir()
  });

  createWindow();
  buildMenu();
  createTray();

  ipcMain.on('backend:restart', () => {
    cancelRestart();
    backend.restart('manual');
  });
  ipcMain.on('backend:cancel-restart', () => cancelRestart());
  ipcMain.on('app:get-status', () => sendStatus());
  ipcMain.on('app:open-browser', () => openInBrowser());
  ipcMain.on('app:open-logs', () => openLogsDir());
  ipcMain.on('app:reload', () => {
    if (win && !win.isDestroyed()) {
      log('手动刷新前端页面');
      win.webContents.reloadIgnoringCache();
    }
  });
  ipcMain.on('app:quit', () => app.quit());

  // 下载（会话导出等）默认保存到系统下载目录
  session.defaultSession.on('will-download', (e, item) => {
    const suggested = item.getFilename();
    dialog
      .showSaveDialog(win, {
        title: '保存文件',
        defaultPath: path.join(app.getPath('downloads'), suggested)
      })
      .then(({ canceled, filePath }) => {
        if (canceled || !filePath) {
          item.cancel();
          return;
        }
        item.setSavePath(filePath);
      });
  });

  backend.startProfileWatch();
  backend.start();
  setupTestControl();
  setupPluginInstallChannel();

  // 单次截图模式（测试用）：后端就绪 8s 后截图退出
  const shotPath = process.env.DSH_DESKTOP_SCREENSHOT;
  if (shotPath) {
    const check = setInterval(() => {
      if (lastStatus && (lastStatus.state === 'running' || lastStatus.state === 'external')) {
        clearInterval(check);
        setTimeout(async () => {
          try {
            const image = await win.webContents.capturePage();
            fs.writeFileSync(shotPath, image.toPNG());
            log(`截图已保存: ${shotPath}`);
          } catch (e) {
            log(`截图失败: ${e.message}`);
          }
          app.quit();
        }, 8000);
      }
    }, 1000);
  }
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

app.on('before-quit', (e) => {
  if (quitting) return;
  e.preventDefault();
  quitting = true;
  log('退出中，停止后端…');
  const timeout = setTimeout(() => {
    log('停止后端超时，强制退出');
    app.exit(0);
  }, 8000);
  if (backend) {
    backend.stop().finally(() => {
      clearTimeout(timeout);
      app.quit();
    });
  } else {
    clearTimeout(timeout);
    app.quit();
  }
});
