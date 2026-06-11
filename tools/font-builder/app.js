'use strict';

const SIZES = [
  { label: 'SMALL', px: 16, dev: 'NSD16.RWF', guj: 'NSG16.RWF', tag: 0 },
  { label: 'MEDIUM', px: 20, dev: 'NSD20.RWF', guj: 'NSG20.RWF', tag: 1 },
  { label: 'LARGE', px: 24, dev: 'NSD24.RWF', guj: 'NSG24.RWF', tag: 2 },
  { label: 'XLARGE', px: 28, dev: 'NSD28.RWF', guj: 'NSG28.RWF', tag: 3 },
];
const DEV_RANGES = [[0x0900, 0x097f], [0xa8e0, 0xa8ff], [0x1cd0, 0x1cff]];
const GUJ_RANGES = [[0x0a80, 0x0aff]];
const MAX_PACK_BYTES = 1024 * 1024;
const MAX_GLYPHS = 8192;
const status = document.getElementById('status');

function log(message) { status.textContent += `\n${message}`; }
function isInRanges(code, ranges) { return ranges.some(([a, b]) => code >= a && code <= b); }
function hasScript(text, ranges) { return [...text].some(ch => isInRanges(ch.codePointAt(0), ranges)); }
function sequencesForScript(text, ranges) {
  const values = new Set();
  for (const [start, end] of ranges) {
    for (let code = start; code <= end; code += 1) values.add(String.fromCodePoint(code));
  }
  const segmenter = new Intl.Segmenter('und', { granularity: 'grapheme' });
  for (const item of segmenter.segment(text)) {
    const cluster = item.segment;
    if (hasScript(cluster, ranges) && new TextEncoder().encode(cluster).length <= 64) values.add(cluster);
  }
  return [...values].sort((a, b) => a.localeCompare(b));
}
async function loadFont(input, family) {
  const file = input.files[0];
  if (!file) return false;
  const bytes = await file.arrayBuffer();
  const face = new FontFace(family, bytes);
  await face.load();
  document.fonts.add(face);
  return true;
}
function renderGlyph(sequence, family, px, alphaThreshold) {
  const probe = document.createElement('canvas').getContext('2d');
  probe.font = `${px}px "${family}"`;
  probe.textBaseline = 'alphabetic';
  const metric = probe.measureText(sequence);
  const ascent = Math.ceil(metric.actualBoundingBoxAscent || px);
  const descent = Math.ceil(metric.actualBoundingBoxDescent || Math.max(2, px / 4));
  const leftPad = Math.ceil(metric.actualBoundingBoxLeft || 0) + 2;
  const rightPad = Math.ceil(metric.actualBoundingBoxRight || metric.width) + 2;
  const width = Math.max(1, leftPad + rightPad);
  const height = Math.max(1, ascent + descent + 4);
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  ctx.clearRect(0, 0, width, height);
  ctx.fillStyle = '#000';
  ctx.font = `${px}px "${family}"`;
  ctx.textBaseline = 'alphabetic';
  const baseline = ascent + 2;
  ctx.fillText(sequence, leftPad, baseline);
  const data = ctx.getImageData(0, 0, width, height).data;
  const stride = Math.ceil(width / 8);
  const bitmap = new Uint8Array(stride * height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      if (data[(y * width + x) * 4 + 3] >= alphaThreshold) bitmap[y * stride + Math.floor(x / 8)] |= 0x80 >> (x % 8);
    }
  }
  return {
    sequence,
    width,
    height,
    advance: Math.max(1, Math.ceil(metric.width)),
    left: -leftPad,
    top: -baseline,
    bitmap,
  };
}
function pushU16(out, value) { out.push(value & 0xff, (value >> 8) & 0xff); }
function pushI16(out, value) { pushU16(out, value & 0xffff); }
function pushU32(out, value) { out.push(value & 0xff, (value >> 8) & 0xff, (value >> 16) & 0xff, (value >> 24) & 0xff); }
function buildPack(scriptTag, size, family, sequences, alphaThreshold) {
  const encoder = new TextEncoder();
  const output = [0x52, 0x57, 0x46, 0x31, scriptTag, size.tag];
  pushU16(output, size.px + 4);
  pushU32(output, sequences.length);
  for (const sequence of sequences) {
    const glyph = renderGlyph(sequence, family, size.px, alphaThreshold);
    const key = encoder.encode(sequence);
    pushU16(output, key.length);
    pushU16(output, glyph.width);
    pushU16(output, glyph.height);
    pushI16(output, glyph.advance);
    pushI16(output, glyph.left);
    pushI16(output, glyph.top);
    pushU32(output, glyph.bitmap.length);
    output.push(...key, ...glyph.bitmap);
  }
  return new Uint8Array(output);
}
function ensurePackBounds(name, bytes, sequenceCount) {
  if (sequenceCount > MAX_GLYPHS) throw new Error(`${name} has ${sequenceCount} clusters; maximum is ${MAX_GLYPHS}. Reduce the corpus.`);
  if (bytes.length > MAX_PACK_BYTES) throw new Error(`${name} is ${bytes.length} bytes; maximum is ${MAX_PACK_BYTES}. Reduce the corpus.`);
}
function yieldToBrowser() { return new Promise(resolve => setTimeout(resolve, 0)); }
function download(name, bytes, type = 'application/octet-stream') {
  const blob = new Blob([bytes], { type });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  document.body.appendChild(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 5000);
}
async function build() {
  status.textContent = 'Loading local files...';
  const corpora = [...document.getElementById('corpora').files];
  if (!corpora.length) throw new Error('Select one or more UTF-8 corpus .txt files.');
  const text = (await Promise.all(corpora.map(file => file.text()))).join('\n');
  const manifest = [];
  const files = [];
  const alphaThreshold = Number.parseInt(document.getElementById('alphaThreshold').value, 10);
  if (![128, 160, 192].includes(alphaThreshold)) throw new Error('Select a supported e-paper alpha threshold.');
  log(`E-paper alpha threshold: ${alphaThreshold}`);
  if (await loadFont(document.getElementById('devanagariFont'), 'RustmixNotoSansDevanagari')) {
    const sequences = sequencesForScript(text, DEV_RANGES);
    log(`Devanagari sequences: ${sequences.length}`);
    for (const size of SIZES) {
      log(`Building ${size.dev}...`);
      const bytes = buildPack(1, size, 'RustmixNotoSansDevanagari', sequences, alphaThreshold);
      ensurePackBounds(size.dev, bytes, sequences.length);
      files.push({ name: size.dev, bytes });
      manifest.push(`DEVANAGARI|${size.label}|${size.dev}`);
      await yieldToBrowser();
    }
  }
  if (await loadFont(document.getElementById('gujaratiFont'), 'RustmixNotoSansGujarati')) {
    const sequences = sequencesForScript(text, GUJ_RANGES);
    log(`Gujarati sequences: ${sequences.length}`);
    for (const size of SIZES) {
      log(`Building ${size.guj}...`);
      const bytes = buildPack(2, size, 'RustmixNotoSansGujarati', sequences, alphaThreshold);
      ensurePackBounds(size.guj, bytes, sequences.length);
      files.push({ name: size.guj, bytes });
      manifest.push(`GUJARATI|${size.label}|${size.guj}`);
      await yieldToBrowser();
    }
  }
  if (!manifest.length) throw new Error('Select at least one local Noto Sans font file.');
  files.unshift({ name: 'FONTS.TXT', bytes: new TextEncoder().encode(`${manifest.join('\n')}\n`) });
  files.push({
    name: 'README.TXT',
    bytes: new TextEncoder().encode('Extract this ZIP and copy FONTS.TXT plus every .RWF file to /RUSTMIX/FONTS on the SD card.\n'),
  });
  log('Packaging one ZIP download...');
  const archive = RustmixZip.storedZip(files);
  download('rustmix-indic-font-pack.zip', archive, 'application/zip');
  log('Done. Extract rustmix-indic-font-pack.zip, then install FONTS.TXT and every .RWF file to /RUSTMIX/FONTS.');
}
document.getElementById('build').addEventListener('click', () => build().catch(error => { status.textContent = `Error: ${error.message}`; }));
