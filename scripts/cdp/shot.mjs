#!/usr/bin/env node
/**
 * 对 dsh 页面截图（可选裁剪区域）。
 *
 *   node scripts/cdp/shot.mjs <out.png>                # 整页
 *   node scripts/cdp/shot.mjs <out.png> x y w h        # 裁剪 (CSS 像素, scale=2)
 *
 * ⚠️ 裁剪参数用**4 个独立数字**传，别传 JSON——PowerShell 会把引号吃掉
 * （实测 `'{"x":0,...}'` 报 `Expected property name or '}' in JSON at position 1`）。
 *
 * 无头 Edge 的启动方式见 cdp.mjs 顶部注释。
 */
import { writeFileSync, statSync } from 'node:fs';
import { connect, dshPage } from './cdp.mjs';

const [, , outPath, ...rest] = process.argv;
if (!outPath) {
  console.error('用法: node scripts/cdp/shot.mjs <out.png> [x y w h]');
  process.exit(2);
}

const nums = rest.map(Number);
let clip = null;
if (nums.length > 0) {
  if (nums.length !== 4 || nums.some((n) => !Number.isFinite(n))) {
    console.error('裁剪参数必须是 4 个数字: x y w h');
    process.exit(2);
  }
  clip = { x: nums[0], y: nums[1], width: nums[2], height: nums[3], scale: 2 };
}

const page = await dshPage();
const { send, close } = await connect(page.webSocketDebuggerUrl);

try {
  const params = { format: 'png', captureBeyondViewport: true };
  if (clip) params.clip = clip;
  const res = await send('Page.captureScreenshot', params);
  const data = res.result?.data;
  if (!data) {
    console.error('截图失败：', JSON.stringify(res).slice(0, 400));
    process.exit(1);
  }
  writeFileSync(outPath, Buffer.from(data, 'base64'));
  console.log(`saved ${outPath} (${statSync(outPath).size} bytes)`);
} finally {
  close();
}
