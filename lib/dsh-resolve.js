'use strict';
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

/**
 * 解析 dsh 启动所需的 node.exe 与 dsh 入口，避免依赖用户手动敲命令。
 *
 * dsh 入口探测顺序：
 *   1. settings.dshBin 显式指定（默认安装成功后会自动写入这里）；
 *   2. PATH 上的 dsh / dsh.cmd（npm 全局安装的 shim）；
 *   3. npx 缓存（%LOCALAPPDATA%\npm-cache\_npx\<hash>\node_modules\@deepseek-ai\dsh\lib\bin.js），
 *      仅作为旧安装的兜底查找；安装本身不再使用 npx。
 */

function pathDirs() {
  return (process.env.PATH || '')
    .split(path.delimiter)
    .map((s) => s.trim())
    .filter(Boolean);
}

function fileExists(p) {
  try {
    return fs.statSync(p).isFile();
  } catch {
    return false;
  }
}

function resolveNode() {
  for (const dir of pathDirs()) {
    for (const name of ['node.exe', 'node']) {
      const p = path.join(dir, name);
      if (fileExists(p)) return p;
    }
  }
  for (const p of [
    'C:\\Program Files\\nodejs\\node.exe',
    'C:\\Program Files (x86)\\nodejs\\node.exe',
    path.join(process.env.APPDATA || '', 'npm', 'node.exe')
  ]) {
    if (fileExists(p)) return p;
  }
  return 'node';
}

/** 扫描 npx 缓存里所有 @deepseek-ai/dsh 安装，按修改时间倒序。 */
function npxCacheCandidates() {
  const roots = new Set();
  if (process.env.LOCALAPPDATA) roots.add(path.join(process.env.LOCALAPPDATA, 'npm-cache', '_npx'));
  roots.add(path.join(os.homedir(), 'AppData', 'Local', 'npm-cache', '_npx'));
  roots.add(path.join(os.homedir(), '.npm', '_npx'));
  const out = [];
  for (const root of roots) {
    let entries = [];
    try {
      entries = fs.readdirSync(root);
    } catch {
      continue;
    }
    for (const entry of entries) {
      const bin = path.join(root, entry, 'node_modules', '@deepseek-ai', 'dsh', 'lib', 'bin.js');
      if (fileExists(bin)) {
        try {
          out.push({ bin, mtime: fs.statSync(bin).mtimeMs });
        } catch {}
      }
    }
  }
  out.sort((a, b) => b.mtime - a.mtime);
  return out;
}

/** PATH 上的 dsh shim（全局安装）。 */
function findPathDsh() {
  for (const dir of pathDirs()) {
    for (const name of ['dsh.cmd', 'dsh.bat', 'dsh.exe', 'dsh']) {
      const p = path.join(dir, name);
      if (fileExists(p)) return p;
    }
  }
  return null;
}

/**
 * 解析 dsh 入口。
 * @returns {{kind:'node', bin:string, source:string} | {kind:'cmd', shim:string, source:string} | null}
 */
function resolveDsh(explicitBin) {
  if (explicitBin) {
    if (fileExists(explicitBin)) {
      return explicitBin.toLowerCase().endsWith('.js')
        ? { kind: 'node', bin: explicitBin, source: 'settings' }
        : { kind: 'cmd', shim: explicitBin, source: 'settings' };
    }
    return null;
  }
  const shim = findPathDsh();
  if (shim) return { kind: 'cmd', shim, source: 'PATH' };
  // 仅作旧 npx 安装的兜底查找（安装本身不再使用 npx）
  const cached = npxCacheCandidates();
  if (cached.length > 0) {
    return { kind: 'node', bin: cached[0].bin, source: `npx-cache (${cached.length} 个候选)` };
  }
  return null;
}

module.exports = { resolveNode, resolveDsh, npxCacheCandidates };
