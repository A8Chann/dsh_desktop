'use strict';
/**
 * 生成 assets/icon.png（256x256，RGBA）。
 * 纯 Node 实现：4x 超采样绘制 + 手工 PNG 编码（zlib + CRC32），无任何依赖。
 * 图案：靛蓝→青色渐变圆角方块 + 白色聊天气泡 + 三点。
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

// ── 几何 ──
const clamp01 = (v) => Math.max(0, Math.min(1, v));

function roundRectSdf(px, py, cx, cy, hw, hh, r) {
  const dx = Math.abs(px - cx) - (hw - r);
  const dy = Math.abs(py - cy) - (hh - r);
  const ax = Math.max(dx, 0);
  const ay = Math.max(dy, 0);
  return Math.hypot(ax, ay) + Math.min(Math.max(dx, dy), 0) - r;
}

function pointInTriangle(px, py, ax, ay, bx, by, cx, cy) {
  const d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by);
  const d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy);
  const d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay);
  const hasNeg = d1 < 0 || d2 < 0 || d3 < 0;
  const hasPos = d1 > 0 || d2 > 0 || d3 > 0;
  return !(hasNeg && hasPos);
}

const lerp = (a, b, t) => a + (b - a) * t;

// ── 绘制（SS = 超采样倍数）──
function drawIcon(size, SS) {
  const W = size * SS;
  const px = new Float64Array(W * W * 4);
  const put = (x, y, r, g, b, a) => {
    const i = (y * W + x) * 4;
    // alpha 合成
    const na = a + px[i + 3] * (1 - a);
    if (na <= 0) return;
    px[i] = (r * a + px[i] * px[i + 3] * (1 - a)) / na;
    px[i + 1] = (g * a + px[i + 1] * px[i + 3] * (1 - a)) / na;
    px[i + 2] = (b * a + px[i + 2] * px[i + 3] * (1 - a)) / na;
    px[i + 3] = na;
  };

  const S = W; // 坐标按 W 归一
  for (let y = 0; y < W; y++) {
    for (let x = 0; x < W; x++) {
      const fx = x / W; // 0..1
      const fy = y / W;

      // 背景圆角方块（0.03 边距）
      const bgSdf = roundRectSdf(fx, fy, 0.5, 0.5, 0.47, 0.47, 0.24);
      const bgAlpha = clamp01(0.5 - bgSdf * S * 2);
      if (bgAlpha <= 0) continue;
      // 靛蓝 → 青色渐变
      const t = fy;
      const r = lerp(79, 34, t);
      const g = lerp(70, 211, t);
      const b = lerp(229, 238, t);
      put(x, y, r, g, b, bgAlpha);

      // 聊天气泡（白色圆角矩形 + 小尾巴）
      let bubbleA = 0;
      const bSdf = roundRectSdf(fx, fy, 0.5, 0.485, 0.185, 0.135, 0.065);
      bubbleA = clamp01(0.5 - bSdf * S * 2);
      if (bubbleA > 0.003) {
        // 尾巴三角形
        const tail = pointInTriangle(fx, fy, 0.355, 0.60, 0.44, 0.60, 0.40, 0.695);
        if (tail) bubbleA = Math.max(bubbleA, 1);
      }
      if (bubbleA > 0) put(x, y, 255, 255, 255, bubbleA * 0.97);

      // 气泡内三点
      for (const [dx, dy] of [[-0.055, 0], [0, 0], [0.055, 0]]) {
        const cx = 0.5 + dx;
        const cy = 0.485 + dy;
        const d = Math.hypot(fx - cx, fy - cy);
        const a = clamp01(0.5 - (d - 0.024) * S * 2);
        if (a > 0) put(x, y, 30, 41, 59, a * (bubbleA > 0.5 ? 1 : 0));
      }
    }
  }

  // 超采样降采样
  const out = Buffer.alloc(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      let r = 0, g = 0, b = 0, a = 0;
      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const i = ((y * SS + sy) * W + (x * SS + sx)) * 4;
          const aa = px[i + 3];
          r += px[i] * aa;
          g += px[i + 1] * aa;
          b += px[i + 2] * aa;
          a += aa;
        }
      }
      const n = SS * SS;
      const oi = (y * size + x) * 4;
      if (a > 0) {
        out[oi] = Math.round(r / a);
        out[oi + 1] = Math.round(g / a);
        out[oi + 2] = Math.round(b / a);
        out[oi + 3] = Math.round(a / n);
      } else {
        out[oi + 3] = 0;
      }
    }
  }
  return out;
}

const size = 256;
const rgba = drawIcon(size, 4);
const png = encodePng(size, size, rgba);
const outPath = path.join(__dirname, '..', 'assets', 'icon.png');
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, png);
console.log(`icon written: ${outPath} (${png.length} bytes)`);
