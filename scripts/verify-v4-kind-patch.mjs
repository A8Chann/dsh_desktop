#!/usr/bin/env node
/**
 * v4 source-kind 补丁验证（oracle = core 自己导出的 assertV4RowAdmission）。
 *
 * 背景：dsh 0.1.7 起 `@deepseek-ai/dsh-session-format-v3-to-v4` 要求
 * message.source.kind 为非空字符串且 !== "plugin"（producer-owned source kind）。
 * 第三方插件若实时发 `{kind:"plugin", plugin:"<name>"}`，整轮提交被拒 →
 * 「本轮运行失败 format v4 message requires a producer-owned source kind」。
 * 修法（参考 deepseek-harness discussion #7455）：kind 改为 `plugin:<原名>`，
 * 与迁移链对未知插件的合成形态（producerKind() 的 `plugin:${plugin}`）一致。
 *
 * 用法：
 *   node scripts/verify-v4-kind-patch.mjs [session.v4.jsonl.zstd ...]
 *
 * 退出码：0 = 全部通过；1 = 有失败项。
 */
import { readFileSync, existsSync } from 'node:fs';
import { zstdDecompressSync } from 'node:zlib';
import { homedir } from 'node:os';
import { join } from 'node:path';

const V3TOV4 = 'file:///C:/Users/HWX/AppData/Roaming/DSH%20Desktop/versions/v18d385b479a67f08/node_modules/@deepseek-ai/dsh-session-format-v3-to-v4/lib/index.js';
const { assertV4RowAdmission } = await import(V3TOV4);

// ── 多帧 zstd 解码（session.jsonl.zstd 是多帧追加，单次解压只吃第一帧）──
// 帧结构解析与 scripts/session-log.mjs::scanZstdFrames 保持一致（那套已实测过）。
const ZSTD_MAGIC = 0xfd2fb528;

function scanZstdFrames(buffer, maxFrames = Number.POSITIVE_INFINITY) {
  const frames = [];
  let offset = 0;
  while (offset < buffer.length) {
    const start = offset;
    if (buffer.length - offset < 4) return { frames, tornStart: start };
    if (buffer.readUInt32LE(offset) !== ZSTD_MAGIC) {
      throw new Error(`corrupt: 位置 ${offset} 不是 zstd magic`);
    }
    offset += 4;
    if (offset === buffer.length) return { frames, tornStart: start };
    const descriptor = buffer.readUInt8(offset);
    offset += 1;
    if ((descriptor & 24) !== 0) throw new Error(`corrupt: 保留位非零 (${offset - 1})`);
    const contentSizeFlag = descriptor >>> 6;
    const singleSegment = (descriptor & 32) !== 0;
    const checksum = (descriptor & 4) !== 0;
    const dictionaryFlag = descriptor & 3;
    const dictionaryBytes = dictionaryFlag === 3 ? 4 : dictionaryFlag;
    const contentSizeBytes = contentSizeFlag === 0 ? (singleSegment ? 1 : 0) : 1 << contentSizeFlag;
    const remainingHeaderBytes = (singleSegment ? 0 : 1) + dictionaryBytes + contentSizeBytes;
    if (buffer.length - offset < remainingHeaderBytes) return { frames, tornStart: start };
    offset += remainingHeaderBytes;
    for (;;) {
      if (buffer.length - offset < 3) return { frames, tornStart: start };
      const blockHeader = buffer.readUIntLE(offset, 3);
      offset += 3;
      const lastBlock = (blockHeader & 1) !== 0;
      const blockType = (blockHeader >>> 1) & 3;
      const blockSize = blockHeader >>> 3;
      if (blockType === 3) throw new Error('corrupt: 保留 block type');
      const payloadBytes = blockType === 1 ? 1 : blockSize;
      if (buffer.length - offset < payloadBytes) return { frames, tornStart: start };
      offset += payloadBytes;
      if (lastBlock) break;
    }
    if (checksum) {
      if (buffer.length - offset < 4) return { frames, tornStart: start };
      offset += 4;
    }
    frames.push({ start, end: offset });
    if (frames.length === maxFrames) return { frames };
  }
  return { frames };
}

function decodeMultiFrameZstd(buffer) {
  const { frames, tornStart } = scanZstdFrames(buffer);
  const parts = [];
  for (const f of frames) {
    try {
      parts.push(zstdDecompressSync(buffer.subarray(f.start, f.end)));
    } catch {
      /* 单帧坏了不影响其余帧 */
    }
  }
  if (tornStart !== undefined) {
    try {
      parts.push(zstdDecompressSync(buffer.subarray(tornStart)));
    } catch {
      /* 写了一半的尾帧，尽力而为 */
    }
  }
  if (parts.length === 0) parts.push(zstdDecompressSync(buffer));
  return Buffer.concat(parts).toString('utf8').split('\n');
}

const PLUGINS = [
  {
    name: '@memtensor/memos-cloud-dsh-plugin',
    file: join(homedir(), '.dsh/profiles/web/node_modules/@memtensor/memos-cloud-dsh-plugin/lib/index.js'),
    oldKind: 'plugin:memos-cloud',
  },
  {
    name: 'dsh-rule-manager',
    file: join(homedir(), '.dsh/profiles/web/node_modules/dsh-rule-manager/lib/index.js'),
    oldKind: 'plugin:dsh-rule-manager',
  },
];

let failures = 0;
const fail = (msg) => { failures += 1; console.error(`  FAIL ${msg}`); };
const pass = (msg) => console.log(`  ok   ${msg}`);

/** 拼一条 user/message 物理行（data 即消息本体，见 assertV4SourceRowAdmission）。 */
function userMessageRow(source) {
  return {
    type: 'user/message',
    seq: 1,
    time: Date.now(),
    data: { id: 'verify-1', role: 'user', content: [{ type: 'text', text: 'x' }], source },
  };
}

function expectThrow(label, row, pattern) {
  try {
    assertV4RowAdmission(row);
    fail(`${label}: 未被拒绝（期望抛错）`);
  } catch (e) {
    if (pattern && !String(e.message).includes(pattern)) {
      fail(`${label}: 抛错信息不含 "${pattern}" → ${e.message}`);
    } else {
      pass(`${label}: 按预期被拒（${String(e.message).slice(0, 60)}…）`);
    }
  }
}

function expectPass(label, row) {
  try {
    assertV4RowAdmission(row);
    pass(`${label}: 通过准入`);
  } catch (e) {
    fail(`${label}: 被拒 → ${e.message}`);
  }
}

// ── 1. source 形态 oracle ──────────────────────────────────────────────
console.log('== 1. source 形态 oracle（core assertV4RowAdmission）==');
expectThrow('旧形态 memos-cloud {kind:"plugin"}', userMessageRow({ kind: 'plugin', plugin: 'memos-cloud', form: 'recall' }), 'producer-owned source kind');
expectThrow('旧形态 rule-manager {kind:"plugin"}', userMessageRow({ kind: 'plugin', plugin: 'dsh-rule-manager' }), 'producer-owned source kind');
expectPass('补丁形态 memos-cloud {kind:"plugin:memos-cloud", form:"recall"}', userMessageRow({ kind: 'plugin:memos-cloud', form: 'recall' }));
expectPass('补丁形态 rule-manager {kind:"plugin:dsh-rule-manager"}', userMessageRow({ kind: 'plugin:dsh-rule-manager' }));

// ── 2. 补丁落盘检查 ───────────────────────────────────────────────────
console.log('== 2. 插件产物补丁落盘检查 ==');
for (const p of PLUGINS) {
  if (!existsSync(p.file)) { fail(`${p.name}: 文件不存在 ${p.file}`); continue; }
  const text = readFileSync(p.file, 'utf8');
  // 先剥掉注释（补丁自带说明注释里含 `kind: "plugin"` 字样，会误报），
  // 只保留整行 // 注释与 /* */ 块注释的移除（写入点不可能在注释里）。
  const code = text
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/^[ \t]*\/\/.*$/gm, '');
  const stale = /kind: *["']plugin["']/.test(code);
  if (stale) fail(`${p.name}: 仍含 kind: "plugin" 写入点`);
  else pass(`${p.name}: 无 kind: "plugin" 残留`);
  if (!text.includes(p.oldKind)) fail(`${p.name}: 未找到新 kind "${p.oldKind}"`);
  else pass(`${p.name}: 新 kind "${p.oldKind}" 已在位`);
}

// ── 3. 活跃 v4 会话日志逐行准入 ───────────────────────────────────────
console.log('== 3. v4 会话日志逐行 assertV4RowAdmission ==');
const files = process.argv.slice(2);
if (files.length === 0) {
  console.log('  （未提供会话文件，跳过）');
} else {
  for (const f of files) {
    if (!existsSync(f)) { fail(`文件不存在 ${f}`); continue; }
    let lines;
    try {
      lines = decodeMultiFrameZstd(readFileSync(f));
    } catch (e) {
      fail(`${f}: 解码失败 → ${e.message}`);
      continue;
    }
    let rejected = 0;
    let firstErr = '';
    for (const line of lines) {
      if (!line.trim()) continue;
      let row;
      try { row = JSON.parse(line); } catch { continue; } // 半行/截断行跳过
      try {
        assertV4RowAdmission(row);
      } catch (e) {
        rejected += 1;
        if (!firstErr) firstErr = `${row.type} seq=${row.seq}: ${e.message}`;
      }
    }
    const base = f.split(/[\\/]/).slice(-2).join('/');
    if (rejected === 0) pass(`${base}: ${lines.length} 行全部通过`);
    else fail(`${base}: ${rejected}/${lines.length} 行被拒，首条：${firstErr}`);
  }
}

console.log(failures === 0 ? '\n全部通过 ✅' : `\n${failures} 项失败 ❌`);
process.exit(failures === 0 ? 0 : 1);
