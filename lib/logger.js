'use strict';
const fs = require('node:fs');
const path = require('node:path');

const MAX_LOG_BYTES = 5 * 1024 * 1024;

/**
 * 简单文件日志：写入 userData/logs/main.log，超 5MB 轮转为 .old。
 */
class Logger {
  constructor(dir) {
    this.dir = dir;
    this.file = path.join(dir, 'main.log');
    try {
      fs.mkdirSync(dir, { recursive: true });
    } catch {}
    this.rotateIfNeeded();
  }

  rotateIfNeeded() {
    try {
      const st = fs.statSync(this.file);
      if (st.size > MAX_LOG_BYTES) {
        fs.copyFileSync(this.file, this.file + '.old');
        fs.truncateSync(this.file, 0);
      }
    } catch {}
  }

  write(level, msg) {
    const line = `${new Date().toISOString()} [${level}] ${msg}`;
    try {
      fs.appendFileSync(this.file, line + '\n');
    } catch {}
    if (process.env.DSH_DESKTOP_VERBOSE) console.log(line);
  }

  info(msg) { this.write('info', msg); }
  warn(msg) { this.write('warn', msg); }
  error(msg) { this.write('error', msg); }
}

module.exports = { Logger };
