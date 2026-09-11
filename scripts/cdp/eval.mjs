#!/usr/bin/env node
/**
 * 在 dsh 页面里求值一段 JS 表达式，打印结果。
 *
 *   node scripts/cdp/eval.mjs <表达式文件.js>
 *
 * ⚠️ 表达式**必须写成文件**再传路径。
 * 直接 `node eval.mjs "<内联表达式>"` 会被 PowerShell 的引号/转义处理搞坏
 * （模板字符串会直接 `SyntaxError: Invalid or unexpected token`），
 * 这个坑实测踩过一次，别再试。
 *
 * 表达式用 IIFE 包起来并 `return` 一个字符串最方便（见下例）：
 *   (() => {
 *     const box = document.querySelector('.cm-bbox')
 *     return box.getBoundingClientRect().width + ' x ' + box.getBoundingClientRect().height
 *   })()
 *
 * 无头 Edge 的启动方式见 cdp.mjs 顶部注释。
 */
import { readFileSync } from 'node:fs';
import { connect, dshPage, evaluate } from './cdp.mjs';

const exprFile = process.argv[2];
if (!exprFile) {
  console.error('用法: node scripts/cdp/eval.mjs <表达式文件.js>');
  process.exit(2);
}

const expression = readFileSync(exprFile, 'utf8');
const page = await dshPage();
const { send, close } = await connect(page.webSocketDebuggerUrl);

try {
  const value = await evaluate(send, expression);
  console.log(typeof value === 'string' ? value : JSON.stringify(value, null, 2));
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
} finally {
  close();
}
