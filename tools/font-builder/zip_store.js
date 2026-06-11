(function (global) {
  'use strict';

  function asBytes(value) {
    if (typeof value === 'string') return new TextEncoder().encode(value);
    if (value instanceof Uint8Array) return value;
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    return new Uint8Array(value);
  }

  function pushU16(out, value) {
    out.push(value & 0xff, (value >>> 8) & 0xff);
  }

  function pushU32(out, value) {
    out.push(
      value & 0xff,
      (value >>> 8) & 0xff,
      (value >>> 16) & 0xff,
      (value >>> 24) & 0xff,
    );
  }

  function concat(parts) {
    const total = parts.reduce((sum, part) => sum + part.length, 0);
    const output = new Uint8Array(total);
    let cursor = 0;
    for (const part of parts) {
      output.set(part, cursor);
      cursor += part.length;
    }
    return output;
  }

  const CRC_TABLE = (() => {
    const table = new Uint32Array(256);
    for (let n = 0; n < 256; n += 1) {
      let value = n;
      for (let k = 0; k < 8; k += 1) {
        value = (value & 1) ? (0xedb88320 ^ (value >>> 1)) : (value >>> 1);
      }
      table[n] = value >>> 0;
    }
    return table;
  })();

  function crc32(bytes) {
    let crc = 0xffffffff;
    for (const byte of bytes) crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
    return (crc ^ 0xffffffff) >>> 0;
  }

  function validateName(name) {
    if (!/^[A-Z0-9._-]+$/.test(name)) throw new Error(`Unsafe ZIP entry name: ${name}`);
  }

  function storedZip(entries) {
    if (!entries.length) throw new Error('Cannot create an empty ZIP archive.');
    const encoder = new TextEncoder();
    const localParts = [];
    const centralParts = [];
    let offset = 0;

    for (const entry of entries) {
      validateName(entry.name);
      const name = encoder.encode(entry.name);
      const data = asBytes(entry.bytes);
      const checksum = crc32(data);

      const local = [];
      pushU32(local, 0x04034b50);
      pushU16(local, 20);
      pushU16(local, 0);
      pushU16(local, 0);
      pushU16(local, 0);
      pushU16(local, 0x0021);
      pushU32(local, checksum);
      pushU32(local, data.length);
      pushU32(local, data.length);
      pushU16(local, name.length);
      pushU16(local, 0);
      const localRecord = concat([new Uint8Array(local), name, data]);
      localParts.push(localRecord);

      const central = [];
      pushU32(central, 0x02014b50);
      pushU16(central, 20);
      pushU16(central, 20);
      pushU16(central, 0);
      pushU16(central, 0);
      pushU16(central, 0);
      pushU16(central, 0x0021);
      pushU32(central, checksum);
      pushU32(central, data.length);
      pushU32(central, data.length);
      pushU16(central, name.length);
      pushU16(central, 0);
      pushU16(central, 0);
      pushU16(central, 0);
      pushU16(central, 0);
      pushU32(central, 0);
      pushU32(central, offset);
      centralParts.push(concat([new Uint8Array(central), name]));
      offset += localRecord.length;
    }

    const localDirectory = concat(localParts);
    const centralDirectory = concat(centralParts);
    const end = [];
    pushU32(end, 0x06054b50);
    pushU16(end, 0);
    pushU16(end, 0);
    pushU16(end, entries.length);
    pushU16(end, entries.length);
    pushU32(end, centralDirectory.length);
    pushU32(end, localDirectory.length);
    pushU16(end, 0);
    return concat([localDirectory, centralDirectory, new Uint8Array(end)]);
  }

  const api = { storedZip, crc32 };
  global.RustmixZip = api;
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
})(typeof window !== 'undefined' ? window : globalThis);
