'use strict';
/**
 * Mock dsh 后端：仅用于自动化自测（DSH_DESKTOP_MOCK=1）。
 * 行为与真实 dsh web 对齐：绑定随机端口并打印 `dsh web: http://127.0.0.1:<port>`。
 * 页面里带 pid，方便截图确认重启后是新进程。
 */
const http = require('node:http');

const server = http.createServer((req, res) => {
  if (req.url === '/' || req.url === '') {
    res.setHeader('content-type', 'text/html; charset=utf-8');
    res.end(
      `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>DSH Mock</title></head>` +
        `<body><div id="root"><h1>Mock dsh backend</h1><p id="pid">pid=${process.pid}</p></div></body></html>`
    );
  } else if (req.url === '/api/health') {
    res.setHeader('content-type', 'application/json');
    res.end(JSON.stringify({ ok: true, pid: process.pid }));
  } else {
    res.statusCode = 404;
    res.end('not found');
  }
});

server.listen(0, '127.0.0.1', () => {
  const port = server.address().port;
  console.log(`dsh web: http://127.0.0.1:${port}`);
  console.log(`MOCK_READY pid=${process.pid} port=${port}`);
});

process.on('SIGTERM', () => {
  try {
    server.close();
  } catch {}
  process.exit(0);
});
process.on('SIGINT', () => process.exit(0));
