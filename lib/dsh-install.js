'use strict';
const { spawn, execFile } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const https = require('node:https');

/**
 * dsh 默认安装（首次找不到 dsh 时兜底）：
 *  - 先同时探测多个 npm 源（registry），量取响应耗时；
 *  - 选「能访问 @deepseek-ai/dsh 且响应最快」的那个源；
 *  - 用 npm（而非 npx）把 @deepseek-ai/dsh 装到全局，并把入口路径回传给调用方，
 *    以便写回 settings.dshBin。
 *
 * 关键实现：直接以 `node <npm-cli.js> <args>` 调用 npm，绕开 Windows 下 .cmd/shell 的
 * 引号转义问题。测速请求支持走代理（HTTP_PROXY / HTTPS_PROXY / npm_config_https_proxy 等），
 * 与用户的网络环境保持一致；运行 npm 时会原样透传进程环境，npm 自身继续用它的代理配置。
 */

// 候选 npm 源（按常见可用性排序，测速后取最快者）
const REGISTRY_CANDIDATES = [
  { name: 'npmmirror（国内）', url: 'https://registry.npmmirror.com/' },
  { name: 'npmjs（官方）', url: 'https://registry.npmjs.org/' },
  { name: '腾讯云镜像', url: 'https://mirrors.cloud.tencent.com/npm/' },
  { name: '华为云镜像', url: 'https://repo.huaweicloud.com/repository/npm/' }
];

// 可选：优先使用 https-proxy-agent（若项目已带此传递依赖），否则直连
let HttpsProxyAgent = null;
try {
  const mod = require('https-proxy-agent');
  HttpsProxyAgent = mod.HttpsProxyAgent || mod;
} catch {
  HttpsProxyAgent = null;
}

/** 从环境变量解析代理地址（npm 与常见大写/小写变量）。 */
function resolveProxyEnv() {
  return (
    process.env.npm_config_https_proxy ||
    process.env.npm_config_proxy ||
    process.env.HTTPS_PROXY ||
    process.env.https_proxy ||
    process.env.HTTP_PROXY ||
    process.env.http_proxy
  );
}

/**
 * 探测一个 npm 源：GET <url>@deepseek-ai/dsh，返回 {ok, ms, url}。
 * 只有 HTTP 200（源上确实有这个包）才算可用，且顺带量出延迟（毫秒）。
 */
function probeRegistry(url, { timeoutMs = 6000, proxyUrl } = {}) {
  return new Promise((resolve) => {
    const base = String(url).replace(/\/+$/, '');
    const pkgUrl = `${base}/@deepseek-ai/dsh`;
    const mod = pkgUrl.startsWith('https') ? https : http;
    const opts = { timeout: timeoutMs };
    if (proxyUrl && HttpsProxyAgent) opts.agent = new HttpsProxyAgent(proxyUrl);
    const start = Date.now();
    let settled = false;
    const finish = (ok, ms) => {
      if (settled) return;
      settled = true;
      try {
        req.destroy();
      } catch {}
      resolve({ ok, ms, url: base });
    };
    const req = mod.get(pkgUrl, opts, (res) => {
      const ms = Date.now() - start;
      if (res.statusCode === 200) {
        res.resume();
        res.on('end', () => finish(true, ms));
        res.on('error', () => finish(false, ms));
      } else {
        res.resume();
        finish(false, ms);
      }
    });
    req.on('timeout', () => finish(false, Date.now() - start));
    req.on('error', () => finish(false, Date.now() - start));
  });
}

/**
 * 并行探测所有候选源，返回「可访问且最快」的那一个。
 * 全部失败时回退到 npm 当前配置的 registry（或官方源），保证不因测速失败而卡死。
 * @returns {Promise<{name, url, ms, fallback}>}
 */
async function pickFastestRegistry(options = {}) {
  const proxyUrl = options.proxyUrl || resolveProxyEnv();
  const timeoutMs = options.timeoutMs || 6000;
  const candidates = options.candidates || REGISTRY_CANDIDATES;
  const results = await Promise.all(
    candidates.map(async (c, index) => {
      const p = await probeRegistry(c.url, { timeoutMs, proxyUrl });
      return { ...p, index, name: c.name };
    })
  );
  const ok = results.filter((r) => r.ok);
  if (ok.length === 0) {
    const fallbackUrl = process.env.npm_config_registry || 'https://registry.npmjs.org/';
    return { name: '默认源', url: fallbackUrl, ms: 0, fallback: true };
  }
  ok.sort((a, b) => a.ms - b.ms);
  const best = ok[0];
  return { name: best.name, url: best.url, ms: best.ms, fallback: false };
}

/**
 * 解析 npm 的 CLI 入口（npm-cli.js），以便用 `node <npm-cli.js> <args>` 调用 npm。
 * 优先取 node 同目录下自带的 npm；找不到返回 null。
 */
function resolveNpmCli(nodeBin) {
  if (nodeBin) {
    const dir = path.dirname(nodeBin);
    for (const p of [
      path.join(dir, 'node_modules', 'npm', 'bin', 'npm-cli.js'),
      path.join(dir, 'lib', 'node_modules', 'npm', 'bin', 'npm-cli.js')
    ]) {
      if (fs.existsSync(p)) return p;
    }
  }
  if (process.env.APPDATA) {
    const p = path.join(process.env.APPDATA, 'npm', 'node_modules', 'npm', 'bin', 'npm-cli.js');
    if (fs.existsSync(p)) return p;
  }
  return null;
}

/** 取 npm 全局安装根目录（`node <npm-cli> root -g`），失败返回 null。 */
function getNpmGlobalPrefix({ nodeBin, npmCli, env, timeoutMs = 20000 }) {
  return new Promise((resolve) => {
    if (!nodeBin || !npmCli) {
      resolve(null);
      return;
    }
    execFile(nodeBin, [npmCli, 'root', '-g'], { env, timeout: timeoutMs, windowsHide: true }, (err, stdout) => {
      if (err) {
        resolve(null);
        return;
      }
      const root = String(stdout || '').trim();
      resolve(root || null);
    });
  });
}

/**
 * 用 npm 全局安装 @deepseek-ai/dsh@latest，使用指定 registry。
 * 返回 Promise<{code, out, err}>；code!=0 表示失败。
 * @param {object} opts - { registry, nodeBin, npmCli, env, onProgress, timeoutMs }
 */
function installDsh({ registry, nodeBin, npmCli, env, onProgress, timeoutMs = 600000 }) {
  return new Promise((resolve, reject) => {
    if (!nodeBin || !npmCli) {
      reject(new Error('无法解析 node/npm-cli，无法安装 dsh'));
      return;
    }
    const reg = String(registry).replace(/\/+$/, '') + '/';
    const args = [
      npmCli,
      'install',
      '-g',
      '@deepseek-ai/dsh@latest',
      '--registry',
      reg,
      '--loglevel',
      'info',
      '--no-fund',
      '--no-audit'
    ];
    const child = spawn(nodeBin, args, { env, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    let out = '';
    let err = '';
    let finished = false;
    const finish = (code) => {
      if (finished) return;
      finished = true;
      resolve({ code, out, err });
    };
    child.stdout.on('data', (c) => {
      const text = c.toString();
      out += text;
      if (onProgress) onProgress(text);
    });
    child.stderr.on('data', (c) => {
      const text = c.toString();
      err += text;
      if (onProgress) onProgress(text);
    });
    child.on('error', (e) => {
      if (!finished) {
        finished = true;
        reject(e);
      }
    });
    child.on('exit', (code) => finish(code));
  });
}

module.exports = {
  REGISTRY_CANDIDATES,
  resolveProxyEnv,
  probeRegistry,
  pickFastestRegistry,
  resolveNpmCli,
  getNpmGlobalPrefix,
  installDsh
};
