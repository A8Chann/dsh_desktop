'use strict';
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');

const DEFAULTS = {
  /** dsh web 监听端口；0 = 由系统分配随机端口 */
  port: 3080,
  /** dsh 后端工作目录（agent 的文件操作根目录）；null → 用户主目录 */
  workspace: null,
  /** 关闭窗口时最小化到托盘而不是退出 */
  closeToTray: false,
  /** 检测到插件变更（package.json / pnpm-lock.yaml 变化）后自动重启后端 */
  autoRestartAfterPluginChange: true,
  /** 自动重启前的可取消倒计时（秒） */
  restartCountdownSec: 6,
  /** 要启动的 dsh profile */
  profile: 'web',
  /** 手动指定 node.exe；null → 自动探测 PATH */
  nodeBin: null,
  /** 手动指定 dsh 入口（bin.js 或 dsh.cmd 的绝对路径）；null → 自动探测 npx 缓存 / PATH */
  dshBin: null
};

function settingsFile(userDataDir) {
  return path.join(userDataDir, 'settings.json');
}

function loadSettings(userDataDir) {
  const merged = { ...DEFAULTS };
  try {
    const raw = JSON.parse(fs.readFileSync(settingsFile(userDataDir), 'utf8'));
    Object.assign(merged, raw);
  } catch {}
  if (typeof merged.workspace !== 'string' || !fs.existsSync(merged.workspace)) {
    merged.workspace = os.homedir();
  }
  if (typeof merged.port !== 'number' || !Number.isFinite(merged.port) || merged.port < 0 || merged.port > 65535) {
    merged.port = DEFAULTS.port;
  }
  return merged;
}

function saveSettings(userDataDir, settings) {
  try {
    fs.mkdirSync(userDataDir, { recursive: true });
    fs.writeFileSync(settingsFile(userDataDir), JSON.stringify(settings, null, 2), 'utf8');
  } catch {}
}

module.exports = { loadSettings, saveSettings, settingsFile, DEFAULTS };
