#!/usr/bin/env node
/**
 * 旧世代会话读取实证：复刻 dsh 0.1.7 真实读取链，验证含 kind:"plugin" 行的
 * 旧 v3 会话能否被迁移打开（读侧表现 B 验证）。
 *
 * 真实路径（@deepseek-ai/dsh-session-persistence-jsonl）：
 *   open(id,'read') → requireStoredLog → loadStoredMigration → prepareStoredMigration
 *     → createSessionFormatCatalogWithChildren(children).createRestore(header, {recovery,validation})
 *     → 逐行 restore.decodeRow(row)（MigratingJsonlRows.consume，无 v4 准入门）
 *     → restore.finish() → restoreReleasedV4Artifact + Session.fromRestore 全量校验
 *
 * 注意：与讨论 #7455 中 Ansonfishing 的 oracle（直接对原始行跑 assertV4RowAdmission）
 * 不同——真实读取链对旧世代走迁移、迁移后才校验，准入门只作用于当前 v4 世代文件。
 *
 * 用法：node scripts/verify-legacy-session-read.mjs <session.v3.jsonl.zstd> ...
 * 退出码：0 = 全部迁移成功；1 = 有失败。
 */
import { readFileSync, existsSync } from 'node:fs';
import { zstdDecompressSync } from 'node:zlib';

const MANAGED = 'C:/Users/HWX/AppData/Roaming/DSH Desktop/versions/v18d385b479a67f08/node_modules/@deepseek-ai';
const catalogMod = await import(`file:///${MANAGED}/dsh-session-format-catalog/lib/index.js`);
const { createSessionFormatCatalogWithChildren } = catalogMod;

// 多帧 zstd 解码（与 scripts/session-log.mjs::scanZstdFrames 同逻辑）
const ZSTD_MAGIC = 0xfd2fb528;
function decodeMultiFrameZstd(buffer) {
  const frames = [];
  let offset = 0;
  while (offset < buffer.length) {
    const start = offset;
    if (buffer.length - offset < 4) return { frames, tornStart: start };
    if (buffer.readUInt32LE(offset) !== ZSTD_MAGIC) throw new Error(`corrupt: 位置 ${offset} 不是 zstd magic`);
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
  }
  const parts = [];
  for (const f of frames) {
    try { parts.push(zstdDecompressSync(buffer.subarray(f.start, f.end))); } catch { /* 坏帧跳过 */ }
  }
  if (parts.length === 0) parts.push(zstdDecompressSync(buffer));
  return { frames, text: Buffer.concat(parts).toString('utf8') };
}

let failures = 0;

/** 深度遍历 artifact，收集所有 message 的 source.kind（含 agent/inbox/spliced 等嵌套）。 */
function collectSourceKinds(value, out) {
  if (Array.isArray(value)) { for (const v of value) collectSourceKinds(v, out); return; }
  if (value && typeof value === 'object') {
    if (typeof value.kind === 'string' && ('plugin' in value || 'callId' in value || 'rpcId' in value || 'form' in value || 'change' in value || 'sections' in value || 'summary' in value)) {
      out.push(value.kind);
    }
    for (const v of Object.values(value)) collectSourceKinds(v, out);
  }
}

for (const file of process.argv.slice(2)) {
  const label = file.split(/[\\/]/).slice(-2).join('/');
  if (!existsSync(file)) { console.error(`  FAIL ${label}: 不存在`); failures += 1; continue; }
  try {
    const { text } = decodeMultiFrameZstd(readFileSync(file));
    const lines = text.split('\n').filter((l) => l.trim().length > 0);
    const header = JSON.parse(lines[0]);
    if (header.version !== 3) {
      console.log(`  skip ${label}: header version=${header.version}（本脚本只测 v3）`);
      continue;
    }
    // 与真实路径一致：无子会话 → children=[]；recovery/validation 与宿主一致
    const catalog = createSessionFormatCatalogWithChildren([]);
    const restore = catalog.createRestore(header, { recovery: 'recoverable', validation: 'current' });
    let pluginKindRows = 0;
    for (const line of lines.slice(1)) {
      const row = JSON.parse(line);
      const kinds = [];
      collectSourceKinds(row.data, kinds);
      if (kinds.includes('plugin')) pluginKindRows += 1;
      restore.decodeRow(row);
    }
    const artifact = restore.finish();
    const kinds = [];
    collectSourceKinds(artifact.events, kinds);
    const residualPlugin = kinds.filter((k) => k === 'plugin').length;
    const version = artifact.header.version;
    const ok = version === 4 && residualPlugin === 0;
    console.log(`  ${ok ? 'ok  ' : 'FAIL'} ${label}: v3→v${version}，${artifact.events.length} 事件；` +
      `原文件含 kind:"plugin" 行 ${pluginKindRows} 条，迁移后残留 ${residualPlugin} 条` +
      `${pluginKindRows > 0 ? `（改写样例：${[...new Set(kinds)].slice(0, 6).join(', ')}）` : ''}`);
    if (!ok) failures += 1;
  } catch (e) {
    console.error(`  FAIL ${label}: ${e.message}`);
    failures += 1;
  }
}

console.log(failures === 0 ? '\n旧会话读取链验证通过 ✅' : `\n${failures} 个文件失败 ❌`);
process.exit(failures === 0 ? 0 : 1);
