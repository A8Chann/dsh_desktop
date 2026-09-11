/**
 * 极简 CDP 客户端（无依赖，用 Node 全局 WebSocket）。
 *
 * 用途：对接无头 Edge，对运行中的 dsh 页面做「不求值截图 / DOM dump / 重载」
 * 这类精确验证——比肉眼截图快，也不需要人盯着浏览器。
 *
 * 启动无头 Edge（本机路径）：
 *   msedge --headless=new --remote-debugging-port=9222 \
 *          --user-data-dir=%TEMP%\edgecdp --window-size=1600,1000 \
 *          "http://127.0.0.1:3080/?token=<token>"
 * token 取自 http://127.0.0.1:19431/status 的 url 字段。
 *
 * ⚠️ 用完只杀命令行含该 --user-data-dir 的 msedge 进程，别误杀用户自己的 Edge。
 */
import http from 'node:http';

const DEFAULT_PORT = Number(process.env.CDP_PORT || 9222);

/** 读取 CDP 的 target 列表。 */
export function targets(port = DEFAULT_PORT) {
  return new Promise((resolve, reject) => {
    http
      .get(`http://127.0.0.1:${port}/json/list`, (res) => {
        let body = '';
        res.on('data', (chunk) => (body += chunk));
        res.on('end', () => {
          try {
            resolve(JSON.parse(body));
          } catch (error) {
            reject(new Error(`CDP target 列表不是合法 JSON: ${error.message}`));
          }
        });
      })
      .on('error', reject);
  });
}

/** 找到 dsh 内容页 target（排除 service_worker / extension 等）。 */
export async function dshPage(port = DEFAULT_PORT) {
  const list = await targets(port);
  const page = list.find((t) => t.type === 'page' && t.url.includes('127.0.0.1:3080'));
  if (!page) {
    throw new Error(
      '找不到 dsh 页面 target。确认无头 Edge 已用 --remote-debugging-port 启动并打开了 3080 端口。'
    );
  }
  return page;
}

/** 连上某个 target，返回 { send, events, close }。 */
export async function connect(wsUrl) {
  const ws = new WebSocket(wsUrl);
  let seq = 0;
  const pending = new Map();
  const events = [];

  ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      pending.get(msg.id)(msg);
      pending.delete(msg.id);
    } else if (msg.method) {
      events.push(msg);
    }
  });

  await new Promise((resolve, reject) => {
    ws.addEventListener('open', resolve);
    ws.addEventListener('error', () => reject(new Error('CDP WebSocket 连接失败')));
  });

  const send = (method, params = {}) =>
    new Promise((resolve) => {
      const id = ++seq;
      pending.set(id, resolve);
      ws.send(JSON.stringify({ id, method, params }));
    });

  return { send, events, close: () => ws.close() };
}

/**
 * 求值一段表达式（表达式请从文件读入——见 eval.mjs 的说明）。
 * 返回反序列化后的值；页面抛错时抛异常。
 */
export async function evaluate(send, expression) {
  const res = await send('Runtime.evaluate', {
    expression,
    returnByValue: true,
    awaitPromise: true,
  });
  const details = res.result?.exceptionDetails;
  if (details) {
    throw new Error(
      '页面求值抛错: ' + (details.exception?.description || details.text || JSON.stringify(details))
    );
  }
  return res.result?.result?.value;
}

/** 等待页面出现指定选择器（轮询，默认最多等 15s）。 */
export async function waitFor(send, selector, timeoutMs = 15000) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const ok = await evaluate(send, `!!document.querySelector(${JSON.stringify(selector)})`);
    if (ok) return true;
    if (Date.now() > deadline) return false;
    await new Promise((r) => setTimeout(r, 400));
  }
}

/** 打印本次连接期间收集到的 console error / 未捕获异常。 */
export function reportErrors(events) {
  const consoleErrors = events
    .filter((e) => e.method === 'Runtime.consoleAPICalled' && e.params.type === 'error')
    .map((e) => (e.params.args || []).map((a) => a.value ?? a.description ?? '').join(' '));
  const exceptions = events
    .filter((e) => e.method === 'Runtime.exceptionThrown')
    .map(
      (e) =>
        e.params.exceptionDetails?.exception?.description || e.params.exceptionDetails?.text || ''
    );
  return { consoleErrors, exceptions };
}
