'use strict';
/**
 * 从 assets/icon.png（256x256，8-bit RGBA）生成 assets/icon.ico，
 * 包含 16/24/32/48/64/128/256 全尺寸（Vista+ 的 PNG 条目）。
 *
 * 纯 Node 实现，零依赖：
 *   1. PNG 解码（8-bit，RGBA/RGB，filter 0-4，非隔行）；
 *   2. box 滤波降采样（预乘 alpha 面积平均，抗锯齿）；
 *   3. 每尺寸重编码为 PNG；
 *   4. 组装标准 ICO 容器（ICONDIR + ICONDIRENTRY + PNG blobs）。
 *
 * 用法：node scripts/make-ico.js   （从 assets/icon.png 生成 assets/icon.ico）
 */
const fs = require('node:fs');
const path = require('node:path');
const zlib = require('node:zlib');

// ── PNG 编码 ──
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, 'ascii');
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crcBuf]);
}

function encodePng(width, height, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type RGBA
  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (stride + 1)] = 0; // filter: none
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0))
  ]);
}

// ── PNG 解码（8-bit，RGBA/RGB，filter 0-4）──
function decodePng(buf) {
  if (buf.readUInt32BE(0) !== 0x89504e47) throw new Error('不是 PNG 文件');
  let pos = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  let interlace = 0;
  const idat = [];
  for (;;) {
    const len = buf.readUInt32BE(pos);
    const type = buf.toString('ascii', pos + 4, pos + 8);
    const data = buf.subarray(pos + 8, pos + 8 + len);
    if (type === 'IHDR') {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
      interlace = data[12];
    } else if (type === 'IDAT') idat.push(data);
    else if (type === 'IEND') break;
    pos += 12 + len;
  }
  if (bitDepth !== 8 || (colorType !== 6 && colorType !== 2) || interlace !== 0) {
    throw new Error(`不支持的 PNG 格式 (depth=${bitDepth} color=${colorType} interlace=${interlace})`);
  }
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const channels = colorType === 6 ? 4 : 3;
  const stride = width * channels;
  const out = Buffer.alloc(width * height * 4);
  let prev = Buffer.alloc(stride);
  let rpos = 0;
  for (let y = 0; y < height; y++) {
    const filter = raw[rpos++];
    const line = Buffer.alloc(stride);
    for (let x = 0; x < stride; x++) {
      const cur = raw[rpos + x];
      const a = x >= channels ? line[x - channels] : 0;
      const b = prev[x];
      const c = x >= channels ? prev[x - channels] : 0;
      let v;
      switch (filter) {
        case 0: v = cur; break;
        case 1: v = cur + a; break;
        case 2: v = cur + b; break;
        case 3: v = cur + ((a + b) >> 1); break;
        case 4: {
          const p = a + b - c;
          const pa = Math.abs(p - a);
          const pb = Math.abs(p - b);
          const pc = Math.abs(p - c);
          v = cur + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c);
          break;
        }
        default: throw new Error(`未知 PNG filter ${filter}`);
      }
      line[x] = v & 0xff;
    }
    for (let i = 0; i < width; i++) {
      const si = i * channels;
      const di = (y * width + i) * 4;
      out[di] = line[si];
      out[di + 1] = line[si + 1];
      out[di + 2] = line[si + 2];
      out[di + 3] = channels === 4 ? line[si + 3] : 255;
    }
    prev = line;
    rpos += stride;
  }
  return { width, height, rgba: out };
}

// ── box 降采样（预乘 alpha 面积平均，抗锯齿）──
function resizeBox(src, sw, sh, dw, dh) {
  const dst = Buffer.alloc(dw * dh * 4);
  for (let ty = 0; ty < dh; ty++) {
    const y0 = (ty * sh) / dh;
    const y1 = ((ty + 1) * sh) / dh;
    for (let tx = 0; tx < dw; tx++) {
      const x0 = (tx * sw) / dw;
      const x1 = ((tx + 1) * sw) / dw;
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;
      let aw = 0;
      const sy0 = Math.floor(y0);
      const sy1 = Math.max(Math.ceil(y1), sy0 + 1);
      const sx0 = Math.floor(x0);
      const sx1 = Math.max(Math.ceil(x1), sx0 + 1);
      for (let sy = sy0; sy < sy1; sy++) {
        const wy = Math.min(y1, sy + 1) - Math.max(y0, sy);
        if (wy <= 0) continue;
        for (let sx = sx0; sx < sx1; sx++) {
          const wx = Math.min(x1, sx + 1) - Math.max(x0, sx);
          if (wx <= 0) continue;
          const w = wx * wy;
          const i = (sy * sw + sx) * 4;
          const alpha = src[i + 3] / 255;
          r += src[i] * alpha * w;
          g += src[i + 1] * alpha * w;
          b += src[i + 2] * alpha * w;
          a += alpha * w;
          aw += w;
        }
      }
      const di = (ty * dw + tx) * 4;
      if (a > 0 && aw > 0) {
        dst[di] = Math.round(r / a);
        dst[di + 1] = Math.round(g / a);
        dst[di + 2] = Math.round(b / a);
        dst[di + 3] = Math.round((a / aw) * 255);
      }
    }
  }
  return dst;
}

// ── ICO 容器 ──
function encodeIco(entries) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type: icon
  header.writeUInt16LE(entries.length, 4);
  const dirs = Buffer.alloc(entries.length * 16);
  let offset = 6 + dirs.length;
  const blobs = [];
  entries.forEach((e, i) => {
    const o = i * 16;
    dirs[o] = e.size >= 256 ? 0 : e.size; // width；0 表示 256
    dirs[o + 1] = e.size >= 256 ? 0 : e.size; // height
    dirs[o + 2] = 0; // color count
    dirs[o + 3] = 0; // reserved
    dirs.writeUInt16LE(1, o + 4); // planes
    dirs.writeUInt16LE(32, o + 6); // bit count
    dirs.writeUInt32LE(e.png.length, o + 8);
    dirs.writeUInt32LE(offset, o + 12);
    blobs.push(e.png);
    offset += e.png.length;
  });
  return Buffer.concat([header, dirs, ...blobs]);
}

// ── 主流程 ──
const SIZES = [16, 24, 32, 48, 64, 128, 256];
const srcPath = path.join(__dirname, '..', 'assets', 'icon.png');
const outPath = path.join(__dirname, '..', 'assets', 'icon.ico');

const src = decodePng(fs.readFileSync(srcPath));
console.log(`source: ${srcPath} (${src.width}x${src.height})`);
const entries = SIZES.map((s) => {
  const rgba = s === src.width ? src.rgba : resizeBox(src.rgba, src.width, src.height, s, s);
  return { size: s, png: encodePng(s, s, rgba) };
});
const ico = encodeIco(entries);
fs.writeFileSync(outPath, ico);
console.log(`icon.ico written: ${outPath} (${ico.length} bytes, sizes ${SIZES.join('/')})`);
