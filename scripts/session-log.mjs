#!/usr/bin/env node
/**
 * dsh 会话日志（zstd）读取工具 —— 多帧解码 + 三种视图。
 *
 * 背景：`~/.dsh/sessions/<workspace>/<session-id>/session.jsonl.zstd` 是**多帧 zstd 追加**文件。
 * Node 的 `zstdDecompressSync` 与流式解压都**只吃第一帧**，直接解只会拿到开头一小段；
 * 必须先按 `28 B5 2F FD`（magic）手工切帧、逐帧解压再拼接。本脚本固化这套逻辑。
 *
 * 用法：
 *   node scripts/session-log.mjs frames <file.zstd>
 *       只看帧统计（帧数 / 行数 / 是否截断），确认解码没问题。
 *
 *   node scripts/session-log.mjs dump <file.zstd>
 *       结构化视图：把每行 JSON 按事件类型（model/selection、request/header、
 *       assistant/message、tool/call、tool/result、turn|step/*、agent/inbox/* …）
 *       摘出关键字段，带 seq / 时间 / T<turn>/<step>。排查会话行为时首选这个。
 *
 *   node scripts/session-log.mjs raw <file.zstd> [--tail N | --range A-B | --seq 1,2,3 | --grep PAT]
 *       原始行视图（不带参数=全部）。行超长会截断；`--grep` 命中即打印该行。
 *
 *   node scripts/session-log.mjs scan <dir|file>... [--grep PAT]
 *       递归扫描目录下所有 *.zstd，逐文件报「行数 / 命中数 + 最后几条命中 + 末尾几行」。
 *       默认关键字聚焦报错与 tool-call 相关（见 DEFAULT_SCAN_PATTERN）。
 *
 * 退出码：0 = 正常；1 = 文件读取/解码失败（详情在 stderr）。
 */
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { zstdDecompressSync } from 'node:zlib';
import { join } from 'node:path';

/** zstd 帧头 magic（小端 uint32）。 */
const ZSTD_MAGIC = 0xfd2fb528;

const DEFAULT_SCAN_PATTERN = /error|commandcode|AI_APICallError|insufficient|tool_calls|toolCall|400/i;

/**
 * 按 zstd 帧结构逐帧切分（**不解压**，只算边界）。
 *
 * 为什么要手写：多帧文件里每一帧自带独立头，Node 只有 `zstdDecompressSync`
 * 这种「整个 buffer 当一帧」的入口。这里按规范解析 frame header：
 * magic(4) + descriptor(1) + [windowSize(0|1)] + [dictId(0|1|2|4)] +
 * [frameContentSize(0|1|2|4|8)]，然后逐个 block header(3 字节) 前进，
 * 直到 lastBlock；有 checksum 位再跳 4 字节。末尾可能是**写了一半的帧**
 * （进程被杀），用 `tornStart` 报出来交给调用方尽力解压。
 *
 * @returns {{ frames: Array<{start:number,end:number}>, tornStart?: number }}
 */
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

/** 解码整个多帧文件，返回 { text, frames, torn }。坏帧跳过（不整体失败）。 */
function decodeSession(file) {
  const bytes = readFileSync(file);
  const { frames, tornStart } = scanZstdFrames(bytes);
  const parts = [];
  for (const f of frames) {
    try {
      parts.push(zstdDecompressSync(bytes.subarray(f.start, f.end)));
    } catch {
      /* 单帧坏了不影响其余帧 */
    }
  }
  let torn = false;
  if (tornStart !== undefined) {
    try {
      parts.push(zstdDecompressSync(bytes.subarray(tornStart)));
    } catch {
      torn = true;
    }
  }
  return { text: Buffer.concat(parts).toString('utf8'), frames, torn };
}

const nonEmptyLines = (text) => text.split('\n').filter((l) => l.trim().length > 0);

/** 取一个参数的值：--flag value */
function flagValue(argv, name) {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : undefined;
}

/** 递归收集目录下所有 *.zstd；传文件则原样返回。 */
function walkZstd(target) {
  const st = statSync(target);
  if (st.isFile()) return target.endsWith('.zstd') ? [target] : [];
  const out = [];
  for (const e of readdirSync(target, { withFileTypes: true })) {
    const p = join(target, e.name);
    if (e.isDirectory()) out.push(...walkZstd(p));
    else if (e.name.endsWith('.zstd')) out.push(p);
  }
  return out;
}

const truncate = (s, n) => (s.length > n ? `${s.slice(0, n)} …TRUNC` : s);

// ── 子命令 ─────────────────────────────────────────────────────────────────

function cmdFrames(file) {
  const { frames, text, torn } = decodeSession(file);
  const lines = nonEmptyLines(text);
  console.log(`${file}\n  frames=${frames.length}  lines=${lines.length}  tornTail=${torn}`);
  if (frames.length) {
    console.log(`  首帧 ${frames[0].start}-${frames[0].end}  末帧 ${frames.at(-1).start}-${frames.at(-1).end}`);
  }
}

function cmdDump(file) {
  const lines = nonEmptyLines(decodeSession(file).text);
  let lastTurn = '';
  for (let i = 0; i < lines.length; i++) {
    let j;
    try {
      j = JSON.parse(lines[i]);
    } catch {
      continue;
    }
    const t = j.type;
    const d = j.data || {};
    let extra = '';
    if (t === 'model/selection') extra = JSON.stringify(d);
    else if (t === 'request/header') extra = 'config=' + JSON.stringify(d.config || {}).slice(0, 200);
    else if (t === 'assistant/message') {
      const m = d.message || {};
      const kinds = (m.content || []).map((b) => b.type).join(',');
      extra = 'blocks=[' + kinds + ']';
      const txt = (m.content || []).find((b) => b.type === 'text');
      if (txt && txt.text) extra += ' text_len=' + txt.text.length;
    } else if (t === 'user/message') {
      const m = d.message || d;
      const src = m.source;
      extra = 'source=' + (src && src.kind);
      if (src && src.kind === 'tool') {
        const blk = (m.content || [])[0];
        if (blk) {
          extra += ' toolCallId=' + blk.toolCallId + ' isError=' + blk.isError +
            ' len=' + JSON.stringify(blk.content || '').length;
        }
      }
      const txt = ((d.content || []).find((b) => b.type === 'text') || {}).text;
      if (txt) extra += ' text="' + txt.slice(0, 120) + '"';
    } else if (t === 'tool/call') extra = d.name;
    else if (t === 'tool/result') {
      extra = 'msg=' + (d.message?.source?.kind || '') +
        ' callId=' + (d.message?.source?.callId || d.message?.content?.[0]?.toolCallId || '');
    } else if (t === 'turn/end' || t === 'turn/start' || t === 'step/end' || t === 'step/start') {
      extra = JSON.stringify(d).slice(0, 140);
    } else if (t === 'agent/inbox/spliced') {
      const ins = d.inserted || [];
      extra = 'inserted=' + ins.length + ' first=' + JSON.stringify(ins[0] || {}).slice(0, 120);
    } else if (/error|fail/i.test(t)) extra = JSON.stringify(d).slice(0, 300);

    if (d.turn !== undefined || d.step !== undefined) {
      const mark = 'T' + (d.turn ?? '?') + '/' + (d.step ?? '?');
      if (mark !== lastTurn) {
        lastTurn = mark;
        extra = mark + ' ' + extra;
      }
    }
    const ts = j.time ? new Date(j.time).toISOString().slice(11, 19) : '--------';
    console.log(String(j.seq ?? '-').padStart(4) + ' ' + ts + ' ' + String(t).padEnd(22) + ' ' + extra);
  }
}

function cmdRaw(file, argv) {
  const lines = nonEmptyLines(decodeSession(file).text);
  const grep = flagValue(argv, '--grep');
  const seqArg = flagValue(argv, '--seq');
  const rangeArg = flagValue(argv, '--range');
  const tailArg = flagValue(argv, '--tail');

  let from = 0;
  let to = lines.length - 1;
  if (tailArg !== undefined) from = Math.max(0, lines.length - Number(tailArg));
  if (rangeArg !== undefined) {
    const [a, b] = rangeArg.split('-').map(Number);
    from = Math.max(0, a);
    to = Math.min(lines.length - 1, Number.isFinite(b) ? b : a);
  }
  const wantSeq = seqArg !== undefined ? new Set(seqArg.split(',').map(Number)) : null;

  for (let i = from; i <= to; i++) {
    if (grep !== undefined && !lines[i].includes(grep)) continue;
    if (wantSeq) {
      let seq;
      try {
        seq = JSON.parse(lines[i]).seq;
      } catch {
        continue;
      }
      if (!wantSeq.has(seq)) continue;
    }
    console.log(`[${i}] ` + truncate(lines[i], 1500));
  }
}

function cmdScan(targets, argv) {
  const pat = flagValue(argv, '--grep');
  const re = pat !== undefined ? new RegExp(pat, 'i') : DEFAULT_SCAN_PATTERN;
  for (const root of targets) {
    for (const f of walkZstd(root)) {
      let lines;
      try {
        lines = nonEmptyLines(decodeSession(f).text);
      } catch (e) {
        console.log('SKIP ' + f + ': ' + e.message);
        continue;
      }
      const hits = [];
      for (let i = 0; i < lines.length; i++) if (re.test(lines[i])) hits.push(i);
      console.log(`### ${f} lines=${lines.length} hits=${hits.length}`);
      if (!hits.length) continue;
      for (const i of hits.slice(-6)) console.log(`  [${i}] ` + truncate(lines[i], 600));
      console.log('  --- tail 3 ---');
      for (let i = Math.max(0, lines.length - 3); i < lines.length; i++) {
        console.log(`  [${i}] ` + truncate(lines[i], 400));
      }
    }
  }
}

// ── 入口 ───────────────────────────────────────────────────────────────────

const USAGE = `用法:
  node scripts/session-log.mjs frames <file.zstd>
  node scripts/session-log.mjs dump   <file.zstd>
  node scripts/session-log.mjs raw    <file.zstd> [--tail N | --range A-B | --seq 1,2,3 | --grep PAT]
  node scripts/session-log.mjs scan   <dir|file>... [--grep PAT]`;

const [cmd, ...rest] = process.argv.slice(2);
try {
  if (cmd === 'frames') {
    if (!rest[0]) throw new Error('缺少文件参数');
    cmdFrames(rest[0]);
  } else if (cmd === 'dump') {
    if (!rest[0]) throw new Error('缺少文件参数');
    cmdDump(rest[0]);
  } else if (cmd === 'raw') {
    if (!rest[0]) throw new Error('缺少文件参数');
    cmdRaw(rest[0], rest);
  } else if (cmd === 'scan') {
    const targets = rest.filter((a) => !a.startsWith('--') && !/^\d+$/.test(a) && a !== flagValue(rest, '--grep'));
    if (!targets.length) throw new Error('缺少目录/文件参数');
    cmdScan(targets, rest);
  } else {
    console.log(USAGE);
    process.exitCode = cmd === undefined ? 0 : 2;
  }
} catch (e) {
  console.error('错误: ' + e.message);
  console.error(USAGE);
  process.exitCode = 1;
}
