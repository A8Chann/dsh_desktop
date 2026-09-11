
import { readFileSync } from 'node:fs';
import { zstdDecompressSync } from 'node:zlib';
const ZSTD_MAGIC = 0xfd2fb528;
function scanZstdFrames(buffer, maxFrames = Infinity) {
  const frames = []; let offset = 0;
  while (offset < buffer.length) {
    const start = offset;
    if (buffer.length - offset < 4) return { frames, tornStart: start };
    if (buffer.readUInt32LE(offset) !== ZSTD_MAGIC) throw new Error('magic');
    offset += 4;
    const descriptor = buffer.readUInt8(offset); offset += 1;
    const contentSizeFlag = descriptor >>> 6;
    const singleSegment = (descriptor & 32) !== 0;
    const checksum = (descriptor & 4) !== 0;
    const dictionaryFlag = descriptor & 3;
    const dictionaryBytes = dictionaryFlag === 3 ? 4 : dictionaryFlag;
    const contentSizeBytes = contentSizeFlag === 0 ? singleSegment ? 1 : 0 : 1 << contentSizeFlag;
    const remainingHeaderBytes = (singleSegment ? 0 : 1) + dictionaryBytes + contentSizeBytes;
    if (buffer.length - offset < remainingHeaderBytes) return { frames, tornStart: start };
    offset += remainingHeaderBytes;
    for (;;) {
      if (buffer.length - offset < 3) return { frames, tornStart: start };
      const blockHeader = buffer.readUIntLE(offset, 3); offset += 3;
      const lastBlock = (blockHeader & 1) !== 0;
      const blockType = blockHeader >>> 1 & 3;
      const blockSize = blockHeader >>> 3;
      if (blockType === 3) throw new Error('reserved');
      const payloadBytes = blockType === 1 ? 1 : blockSize;
      if (buffer.length - offset < payloadBytes) return { frames, tornStart: start };
      offset += payloadBytes;
      if (lastBlock) break;
    }
    if (checksum) { if (buffer.length - offset < 4) return { frames, tornStart: start }; offset += 4; }
    frames.push({ start, end: offset });
  }
  return { frames };
}
const file = process.argv[2];
const bytes = readFileSync(file);
const { frames } = scanZstdFrames(bytes);
const parts = [];
for (const f of frames) { try { parts.push(zstdDecompressSync(bytes.subarray(f.start, f.end))); } catch { } }
const plain = Buffer.concat(parts).toString('utf8');
const lines = plain.split('\n').filter(l => l.trim());
const messages = [];
const seq = [];
for (const l of lines) {
  let j; try { j = JSON.parse(l); } catch { continue; }
  if (!j.data) continue;
  if (j.type === 'assistant/message' && j.data.message) { messages.push(j.data.message); seq.push(j.seq); }
  else if (j.type === 'user/message' && j.data && j.data.content) { messages.push(j.data); seq.push(j.seq); }
  else if (j.type === 'tool/result' && j.data && j.data.message && j.data.message.content) { messages.push(j.data.message); seq.push(j.seq); }
}
console.log('rebuilt messages:', messages.length);
function pairedToolCalls(messages) {
  const callIds = new Set(); const names = new Map(); const resultIds = new Set();
  for (const message of messages) for (const block of message.content) {
    if (message.role === 'assistant' && block.type === 'tool-call') { callIds.add(block.id); names.set(block.id, block.name); }
    if (block.type === 'tool-result') resultIds.add(block.toolCallId);
  }
  return { ids: new Set([...callIds].filter(id => resultIds.has(id))), names };
}
function convertCurrent(messages) {
  const out = [];
  const { ids: paired } = pairedToolCalls(messages);
  for (const message of messages) {
    if (message.role === 'system') continue;
    if (message.role === 'user' && message.source.kind !== 'tool') {
      const parts = [];
      for (const block of message.content) { if (block.type === 'text') parts.push(block.text); else if (block.type === 'image') parts.push('[IMG]'); }
      if (parts.length === 0) continue;
      out.push({ role: 'user', content: parts.length === 1 ? parts[0] : parts });
      continue;
    }
    if (message.role === 'assistant') {
      const text = message.content.filter(b => b.type === 'text').map(b => b.text).join('');
      const reasoning = message.content.filter(b => b.type === 'reasoning').map(b => b.text).join('');
      const toolCalls = message.content.filter(b => b.type === 'tool-call' && paired.has(b.id)).map(b => ({ id: b.id, type: 'function', function: { name: b.name, arguments: b.arguments } }));
      if (text === '' && reasoning === '' && toolCalls.length === 0) continue;
      const assistant = { role: 'assistant', content: text === '' ? null : text };
      if (reasoning !== '') assistant.reasoning_content = reasoning;
      if (toolCalls.length > 0) assistant.tool_calls = toolCalls;
      out.push(assistant);
      continue;
    }
    if (message.role === 'user' && message.source.kind === 'tool') {
      const block = message.content[0];
      if (!block || block.type !== 'tool-result' || !paired.has(block.toolCallId)) continue;
      const images = (block.content || []).filter(b => b.type === 'image');
      const t = (block.content || []).filter(b => b.type === 'text').map(b => b.text).join('');
      out.push({ role: 'tool', tool_call_id: block.toolCallId, content: t });
      if (images.length > 0) out.push({ role: 'user', content: '[IMG_CARRY:' + block.toolCallId + ']' });
    }
  }
  return out;
}
function convertFixed(messages) {
  const out = [];
  const pendingImages = [];
  const flush = () => { for (const m of pendingImages) out.push(m); pendingImages.length = 0; };
  const { ids: paired } = pairedToolCalls(messages);
  for (const message of messages) {
    if (message.role === 'system') continue;
    if (message.role === 'user' && message.source.kind !== 'tool') {
      flush();
      const parts = [];
      for (const block of message.content) { if (block.type === 'text') parts.push(block.text); else if (block.type === 'image') parts.push('[IMG]'); }
      if (parts.length === 0) continue;
      out.push({ role: 'user', content: parts.length === 1 ? parts[0] : parts });
      continue;
    }
    if (message.role === 'assistant') {
      flush();
      const text = message.content.filter(b => b.type === 'text').map(b => b.text).join('');
      const reasoning = message.content.filter(b => b.type === 'reasoning').map(b => b.text).join('');
      const toolCalls = message.content.filter(b => b.type === 'tool-call' && paired.has(b.id)).map(b => ({ id: b.id, type: 'function', function: { name: b.name, arguments: b.arguments } }));
      if (text === '' && reasoning === '' && toolCalls.length === 0) continue;
      const assistant = { role: 'assistant', content: text === '' ? null : text };
      if (reasoning !== '') assistant.reasoning_content = reasoning;
      if (toolCalls.length > 0) assistant.tool_calls = toolCalls;
      out.push(assistant);
      continue;
    }
    if (message.role === 'user' && message.source.kind === 'tool') {
      const block = message.content[0];
      if (!block || block.type !== 'tool-result' || !paired.has(block.toolCallId)) continue;
      const images = (block.content || []).filter(b => b.type === 'image');
      const t = (block.content || []).filter(b => b.type === 'text').map(b => b.text).join('');
      out.push({ role: 'tool', tool_call_id: block.toolCallId, content: t });
      if (images.length > 0) pendingImages.push({ role: 'user', content: '[IMG_CARRY]' });
    }
  }
  flush();
  return out;
}
function validate(msgs) {
  const problems = [];
  let open = null;
  for (let i = 0; i < msgs.length; i++) {
    const m = msgs[i];
    if (open) {
      if (m.role === 'tool') {
        if (open.required.has(m.tool_call_id)) { open.required.delete(m.tool_call_id); if (open.required.size === 0) open = null; }
      } else {
        if (open.required.size > 0) problems.push('msg#' + i + ' (' + m.role + '): 缺 ' + [...open.required].join(','));
        open = null;
      }
    }
    if (m.role === 'assistant' && m.tool_calls && m.tool_calls.length > 0) {
      open = { required: new Set(m.tool_calls.map(t => t.id)) };
    }
  }
  if (open && open.required.size > 0) problems.push('末尾缺: ' + [...open.required].join(','));
  return problems;
}
const cutoff = 253;
const hist = messages.filter((_, i) => seq[i] <= cutoff);
const cur = convertCurrent(hist);
const fix = convertFixed(hist);
console.log('历史最后 4 条源消息:');
for (const m of hist.slice(-4)) console.log('  src role=' + m.role + ' source=' + (m.source && m.source.kind) + ' blocks=' + (m.content || []).map(b => b.type).join(','));
console.log('--- 当前逻辑: assistant 后 6 条 ---');
const ai = cur.findIndex(m => m.role === 'assistant' && m.tool_calls && m.tool_calls.length === 2);
for (const m of cur.slice(ai, ai + 6)) console.log('  ' + JSON.stringify(m).slice(0, 150));
console.log('当前逻辑校验问题:', JSON.stringify(validate(cur)));
console.log('--- 修复逻辑: assistant 后 6 条 ---');
const ai2 = fix.findIndex(m => m.role === 'assistant' && m.tool_calls && m.tool_calls.length === 2);
for (const m of fix.slice(ai2, ai2 + 6)) console.log('  ' + JSON.stringify(m).slice(0, 150));
console.log('修复逻辑校验问题:', JSON.stringify(validate(fix)));
const cut38 = messages.filter((_, i) => seq[i] <= 237);
console.log('单 read_image 历史校验(当前逻辑):', JSON.stringify(validate(convertCurrent(cut38))));
