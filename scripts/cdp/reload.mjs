#!/usr/bin/env node
/**
 * 重载 dsh 页面（绕过缓存）并收集 console error / 未捕获异常。
 *
 *   node scripts/cdp/reload.mjs [等待选择器] [超时毫秒]
 *
 * 默认等待 `.cm-footer-stack`（cost-meter 侧边栏底栏）出现，超时 15000ms。
 * 用于「改完客户端补丁 / 皮肤后，验证页面能正常起来且无报错」这一步。
 *
 * 无头 Edge 的启动方式见 cdp.mjs 顶部注释。
 */
import { connect, dshPage, evaluate, reportErrors, waitFor } from './cdp.mjs';

const selector = process.argv[2] || '.cm-footer-stack';
const timeoutMs = Number(process.argv[3] || 15000);

const page = await dshPage();
const { send, events, close } = await connect(page.webSocketDebuggerUrl);

try {
  await send('Console.enable');
  await send('Runtime.enable');
  await send('Page.enable');
  await send('Page.reload', { ignoreCache: true });

  const ready = await waitFor(send, selector, timeoutMs);
  console.log(`等待 ${selector}: ${ready ? 'READY' : 'TIMEOUT'}`);

  // 给客户端模块一点时间把首屏副作用跑完，再收尾统计报错。
  await new Promise((r) => setTimeout(r, 1500));

  const { consoleErrors, exceptions } = reportErrors(events);
  console.log('console errors:', JSON.stringify(consoleErrors, null, 1));
  console.log('exceptions:', JSON.stringify(exceptions, null, 1));

  if (!ready || consoleErrors.length > 0 || exceptions.length > 0) process.exitCode = 1;
} finally {
  close();
}
