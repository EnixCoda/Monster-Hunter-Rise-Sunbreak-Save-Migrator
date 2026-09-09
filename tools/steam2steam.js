#!/usr/bin/env node
// =============================================================================
// MHRise/Sunbreak Steam -> Steam save migration pipeline
//
// Migrates one character from a source Steam account onto a target Steam
// account, using the same class-copy approach as the Switch -> Steam tool,
// plus the three account/identity fixes that the game validates on load:
//
//   1. class copy       migrate_x: all 34 HASHES classes from the source slot
//                       into the target slot template, re-keyed to the target.
//   2. SteamID64        every occurrence of the source account id inside the
//                       slot is rewritten to the target account id.
//   3. char GUID        the 16-byte field e40fc0cd (shared by sys hunter entry
//                       and slot) is set to the target sys entry's GUID, so
//                       the game can pair the slot with the title-screen entry.
//   4. sys entry        the target sys hunter entry gets the source entry's
//                       display values (name/HR/MR/money/time/array) and the
//                       whole appearance class (gender/face/hair/voice).
//                       Its own GUID and link-derived enums are preserved.
//
// USAGE (module)
//   const { migrateSteam2Steam } = require('./tools/steam2steam.js');
//   const { slot, sys, report } = migrateSteam2Steam({
//     srcSys, srcSlot, srcId, srcCurve,       // source account files + id
//     tgtSys, tgtSlot, tgtId, tgtCurve,       // target account files + id
//     srcCharIdx: 1, tgtCharIdx: 0,           // hunter entry indices
//   });
//
// USAGE (CLI)
//   node tools/steam2steam.js --config cfg.json
//   node tools/steam2steam.js \
//     --src-sys src/data00-1.bin --src-slot src/data002Slot.bin \
//     --src-id <source-steamid64> --src-curve 107 --src-char 1 \
//     --tgt-sys tgt/data00-1.bin --tgt-slot tgt/data001Slot.bin \
//     --tgt-id <target-steamid64> --tgt-curve 77 --tgt-char 0 \
//     --out-dir out/
// =============================================================================
'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const wasm = require(path.join(__dirname, '..', 'wasm', 'pkgnode', 'mhsave_wasm.js'));

const U8 = b => new Uint8Array(b);
const md5 = b => crypto.createHash('md5').update(b).digest('hex');

// --- murmur3 (field-name hash), same seed as ree-lib -------------------------
function murmur3(buf) {
  let h = 0xffffffff >>> 0;
  const c1 = 0xcc9e2d51, c2 = 0x1b873593;
  const len = buf.length;
  let i = 0;
  while (i + 4 <= len) {
    let k = (buf[i] | buf[i + 1] << 8 | buf[i + 2] << 16 | buf[i + 3] << 24) >>> 0;
    k = Math.imul(k, c1); k = (k << 15) | (k >>> 17); k = Math.imul(k, c2);
    h ^= k; h = ((h << 13) | (h >>> 19)) >>> 0; h = (Math.imul(h, 5) + 0xe6546b64) >>> 0;
    i += 4;
  }
  const rem = len - i;
  let k = 0;
  if (rem === 3) k = (buf[i + 2] << 16) | (buf[i + 1] << 8) | buf[i];
  else if (rem === 2) k = (buf[i + 1] << 8) | buf[i];
  else if (rem === 1) k = buf[i];
  if (rem > 0) { k = Math.imul(k, c1); k = (k << 15) | (k >>> 17); k = Math.imul(k, c2); h ^= k; }
  h ^= len; h ^= h >>> 16; h = Math.imul(h, 0x85ebca6b) >>> 0;
  h ^= h >>> 13; h = Math.imul(h, 0xc2b2ae35) >>> 0; h ^= h >>> 16;
  return h >>> 0;
}

// --- field/class constants ---------------------------------------------------
const LOADINFO_CLS = 0xe6a69e8a;          // snow.LoadInfoData
const HUNTER_ARRAY_FIELD = 0x01b05969;    // _HunterArray
const HUNTER_NAME_FIELD = 0xf79f3af6;     // _HunterName
const CHAR_GUID_FIELD = 0xe40fc0cd;       // 16-byte per-character GUID (sys <-> slot)
const APPEARANCE_FIELD = 0xf395dd85;      // -> eec7904b appearance class (gender/face/...)
// display values copied from the source hunter entry
const HUNTER_VALUE_FIELDS = [
  0x5653c091, // HunterRank
  0x4e428685, // MasterRank
  0x34b8ff90, // money / play value
  0x26d05f4a,
  0xdb62caf9,
  0x74c561bf, // last-save time (f64)
  0xd52a810f, // 18 x s32 progress array
  0xd7480e2f,
];
// link-derived enum fields inside the hunter entry (must stay = target link << 8)
const LINK_ENUM_FIELDS = [0x9eb56094, 0xed59430c, 0x8a471883, 0xf4deaf65];

// --- minimal ree save reader (payload starts at offset 16) -------------------
class R {
  constructor(b, p) { this.b = b; this.p = p; }
  u32() { const v = this.b.readUInt32LE(this.p); this.p += 4; return v; }
  i32() { const v = this.b.readInt32LE(this.p); this.p += 4; return v; }
  skip(n) { this.p += n; }
  align(a) { if (a <= 1) return; this.p = (this.p + a - 1) & ~(a - 1); }
}
function readSized(r, ft, size) {
  if (size !== 1) r.align(size);
  switch (ft) {
    case 0x1: r.skip(size); break;
    case 0x2: case 0x3: case 0x4: case 0x0d: r.skip(1); break;
    case 0x5: case 0x6: case 0x0e: r.skip(2); break;
    case 0x7: case 0x8: case 0x0b: r.skip(4); break;
    case 0x9: case 0xa: case 0x0c: r.skip(8); break;
    case -1: readArray(r); break;
    case 0x10: r.skip(size); break;
    default: throw new Error('bad ft ' + ft);
  }
}
function readValue(r, ft) {
  const start = r.p;
  if (ft === -1) { const a = readArray(r); return { kind: 'array', start, end: r.p, array: a }; }
  if (ft === 0x11) { const c = readClass(r); return { kind: 'class', start, end: c.end, cls: c }; }
  if (ft === 0x0f) {
    r.align(4); const size = r.u32(); const vstart = r.p; r.skip(size * 2);
    return { kind: 'str', start, vstart, len: size * 2 };
  }
  r.align(4); const size = r.u32(); r.align(size);
  const vstart = r.p; readSized(r, ft, size);
  return { kind: 'scalar', start, vstart, len: r.p - vstart, size };
}
function readArray(r) {
  r.align(4);
  const mt = r.i32(), ms = r.u32(), len = r.u32(), at = r.i32();
  const elems = [];
  if (at === 1) { const m = r.u32(); if (m === 0xffeeffee) r.skip(len * 4); else r.p -= 4; }
  for (let i = 0; i < len; i++) {
    if (at === 0) {
      if (mt === 0x0f) { r.align(4); const s = r.u32(); r.skip(s * 2); elems.push(null); }
      else { readSized(r, mt, ms); elems.push(null); }
    } else elems.push(readClass(r));
  }
  r.align(4);
  return { mt, ms, len, at, elems };
}
function readField(r) {
  const start = r.p;
  const hash = r.u32();
  const ft = r.i32();
  const val = readValue(r, ft);
  r.align(4);
  return { hash, ft, start, val };
}
function readClass(r) {
  const start = r.p;
  const nf = r.u32(), hash = r.u32();
  const fields = [];
  for (let i = 0; i < nf; i++) fields.push(readField(r));
  return { kind: 'class', hash, start, end: r.p, fields };
}
function parseTopLevel(plain) {
  const r = new R(plain, 16);
  const out = [];
  while (r.b.length - r.p >= 4) {
    const h = r.u32();
    try { out.push({ h, cls: readClass(r) }); } catch (e) { break; }
    if (r.p >= plain.length - 7) break;
  }
  return out;
}
function getHunterArray(plain) {
  for (const top of parseTopLevel(plain)) {
    if (top.cls.hash !== LOADINFO_CLS) continue;
    for (const f of top.cls.fields) {
      if (f.hash === HUNTER_ARRAY_FIELD && f.val.kind === 'array') return f.val.array;
    }
  }
  return null;
}
function hunterEntryFields(plain, idx) {
  const arr = getHunterArray(plain);
  if (!arr || !arr.elems || !arr.elems[idx]) return null;
  const map = {};
  for (const f of arr.elems[idx].fields) map[f.hash] = f;
  return map;
}
function arrayValueRange(plain, f) {
  const r = new R(plain, f.start + 8);
  r.align(4);
  const mt = r.i32(), ms = r.u32(), len = r.u32(), at = r.i32();
  const vstart = r.p;
  if (at === 1) { const m = r.u32(); if (m === 0xffeeffee) r.skip(len * 4); else r.p -= 4; }
  for (let i = 0; i < len; i++) {
    if (at === 0) {
      if (mt === 0x0f) { r.align(4); const s = r.u32(); r.skip(s * 2); }
      else readSized(r, mt, ms);
    } else readClass(r);
  }
  return { start: vstart, end: r.p };
}
// --- helpers -----------------------------------------------------------------
function decrypt(buf, id, curve) { return Buffer.from(wasm.decrypted_bytes(U8(buf), BigInt(id), curve)); }
function patchBytes(buf, id, curve, offset, bytes) {
  return Buffer.from(wasm.patch_bytes(U8(buf), BigInt(id), curve, offset, U8(bytes)));
}
function patchSteamIds(slot, srcId, tgtId, tgtCurve) {
  const plain = decrypt(slot, tgtId, tgtCurve);
  const src = Buffer.alloc(8); src.writeBigUInt64LE(BigInt(srcId));
  const tgt = Buffer.alloc(8); tgt.writeBigUInt64LE(BigInt(tgtId));
  const offsets = [];
  let i = plain.indexOf(src);
  while (i >= 0) { offsets.push(i); i = plain.indexOf(src, i + 1); }
  if (!offsets.length) throw new Error('source SteamID not found in migrated slot');
  let out = slot;
  for (const off of offsets) out = patchBytes(out, tgtId, tgtCurve, off - 16, tgt);
  return { slot: out, offsets };
}
function findGuidField(plain) {
  const pat = Buffer.alloc(4); pat.writeUInt32LE(CHAR_GUID_FIELD);
  let i = plain.indexOf(pat);
  while (i >= 0) {
    try {
      const r = new R(plain, i);
      const f = readField(r);
      if (f.ft === 0x10 && f.val.kind === 'scalar') {
        return { offset: f.val.vstart, len: f.val.len };
      }
    } catch (e) { /* keep scanning */ }
    i = plain.indexOf(pat, i + 1);
  }
  return null;
}
function guidValueAt(plain, field) { return plain.slice(field.offset, field.offset + field.len); }

// --- core --------------------------------------------------------------------
function migrateSteam2Steam(opts) {
  const {
    srcSys, srcSlot, srcId, srcCurve,
    tgtSys, tgtSlot, tgtId, tgtCurve,
    srcCharIdx = 0, tgtCharIdx = 0,
  } = opts;
  for (const [k, v] of Object.entries({ srcSys, srcSlot, tgtSys, tgtSlot })) {
    if (!v) throw new Error('missing input: ' + k);
  }
  if (srcId === undefined || tgtId === undefined) throw new Error('missing srcId/tgtId');

  const srcPlain = decrypt(srcSys, srcId, srcCurve);
  const tgtPlain = decrypt(tgtSys, tgtId, tgtCurve);
  const srcEntry = hunterEntryFields(srcPlain, srcCharIdx);
  const tgtEntry = hunterEntryFields(tgtPlain, tgtCharIdx);
  if (!srcEntry) throw new Error('source hunter entry ' + srcCharIdx + ' not found');
  if (!tgtEntry) throw new Error('target hunter entry ' + tgtCharIdx + ' not found');

  const report = { steps: [], srcCharIdx, tgtCharIdx };

  // 1) copy all HASHES classes into the target template, re-keyed to target
  const mig = wasm.migrate_x(U8(srcSlot), BigInt(srcId), U8(tgtSlot), U8(tgtSys), BigInt(tgtId), srcCurve, tgtCurve);
  let slot = Buffer.from(mig.slot);
  report.steps.push('migrate_x: copied ' + mig.copied + ' classes');

  // 2) rewrite the source account SteamID64 occurrences -> target
  {
    const r = patchSteamIds(slot, srcId, tgtId, tgtCurve);
    slot = r.slot;
    report.steamIdPatches = r.offsets.length;
    report.steps.push('steamid: patched ' + r.offsets.length + ' occurrence(s)');
  }

  // 3) set the slot's character GUID to the target sys entry's GUID
  {
    const tgtGuidField = findGuidField(tgtPlain);
    if (!tgtGuidField) throw new Error('target sys char GUID field not found');
    const tgtGuid = guidValueAt(tgtPlain, tgtGuidField);
    const slotPlain = decrypt(slot, tgtId, tgtCurve);
    const slotGuidField = findGuidField(slotPlain);
    if (!slotGuidField) throw new Error('slot char GUID field not found');
    if (slotGuidField.len !== tgtGuid.length) throw new Error('GUID size mismatch');
    slot = patchBytes(slot, tgtId, tgtCurve, slotGuidField.offset - 16, tgtGuid);
    report.targetGuid = tgtGuid.toString('hex');
    report.steps.push('guid: slot <- ' + report.targetGuid);
  }

  // 4) sys: copy display values + appearance class from source entry
  {
    let sys = Buffer.from(tgtSys);
    const patches = [];
    for (const h of HUNTER_VALUE_FIELDS) {
      const sf = srcEntry[h], tf = tgtEntry[h];
      if (!sf || !tf) { report.steps.push('warn: field 0x' + h.toString(16) + ' missing'); continue; }
      if (sf.val.kind === 'scalar' && tf.val.kind === 'scalar') {
        const sb = srcPlain.slice(sf.val.vstart, sf.val.vstart + sf.val.len);
        if (sf.val.len !== tf.val.len) throw new Error('field size mismatch 0x' + h.toString(16));
        patches.push([tf.val.vstart - 16, sb]);
      } else if (sf.val.kind === 'array' && tf.val.kind === 'array') {
        const sr = arrayValueRange(srcPlain, sf), tr = arrayValueRange(tgtPlain, tf);
        const sb = srcPlain.slice(sr.start, sr.end);
        if (sb.length !== tr.end - tr.start) throw new Error('array size mismatch 0x' + h.toString(16));
        patches.push([tr.start - 16, sb]);
      }
    }
    // name (only when the encoded length matches, so no bytes shift)
    {
      const sf = srcEntry[HUNTER_NAME_FIELD], tf = tgtEntry[HUNTER_NAME_FIELD];
      if (sf && tf && sf.val.kind === 'str' && tf.val.kind === 'str') {
        if (sf.val.len === tf.val.len) {
          patches.push([tf.val.vstart - 16, srcPlain.slice(sf.val.vstart, sf.val.vstart + sf.val.len)]);
          report.steps.push('name: copied (same length)');
        } else {
          report.steps.push('warn: name length differs (' + (sf.val.len / 2) + ' vs ' + (tf.val.len / 2) + ' chars) - rename in game or use a length-neutral patch');
        }
      }
    }
    // appearance class (gender/face/hair/voice/colors)
    {
      const sf = srcEntry[APPEARANCE_FIELD], tf = tgtEntry[APPEARANCE_FIELD];
      if (!sf || !tf || sf.val.kind !== 'class' || tf.val.kind !== 'class') {
        throw new Error('appearance class not found in hunter entries');
      }
      const sb = srcPlain.slice(sf.val.start, sf.val.end);
      if (sb.length !== tf.val.end - tf.val.start) throw new Error('appearance size mismatch');
      patches.push([tf.val.start - 16, sb]);
      report.steps.push('appearance: copied ' + sb.length + ' bytes');
    }
    for (const [off, buf] of patches) sys = patchBytes(sys, tgtId, tgtCurve, off, buf);
    report.steps.push('sys: applied ' + patches.length + ' patch(es)');
    report.sys = sys;
  }

  // 5) verify
  {
    const srcRank = wasm.slot_rank(U8(srcSlot), BigInt(srcId), srcCurve);
    const outRank = wasm.slot_rank(U8(slot), BigInt(tgtId), tgtCurve);
    report.srcRank = srcRank && [srcRank[0], srcRank[1]];
    report.outRank = outRank && [outRank[0], outRank[1]];
    if (!outRank || outRank[0] !== srcRank[0] || outRank[1] !== srcRank[1]) {
      throw new Error('verify failed: rank mismatch');
    }
    const outPlain = decrypt(slot, tgtId, tgtCurve);
    const srcIdPat = Buffer.alloc(8); srcIdPat.writeBigUInt64LE(BigInt(srcId));
    if (outPlain.indexOf(srcIdPat) >= 0) throw new Error('verify failed: source SteamID still present');
    const outGuid = findGuidField(outPlain);
    const tgtGuid = findGuidField(decrypt(tgtSys, tgtId, tgtCurve));
    if (outGuid && tgtGuid &&
        !guidValueAt(outPlain, outGuid).equals(guidValueAt(decrypt(tgtSys, tgtId, tgtCurve), tgtGuid))) {
      throw new Error('verify failed: char GUID mismatch');
    }
    report.outNames = wasm.hunter_names(U8(report.sys), BigInt(tgtId), tgtCurve);
    report.steps.push('verify: rank ' + JSON.stringify(report.outRank) + ', names ' + JSON.stringify(report.outNames));
  }

  return { slot, sys: report.sys, report };
}

// --- CLI ---------------------------------------------------------------------
function main(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 2) {
    const k = argv[i].replace(/^--/, '').replace(/-([a-z])/g, (_, c) => c.toUpperCase());
    args[k] = argv[i + 1];
  }
  if (args.config) Object.assign(args, JSON.parse(fs.readFileSync(args.config, 'utf8')));
  const rd = p => fs.readFileSync(p);
  const { slot, sys, report } = migrateSteam2Steam({
    srcSys: rd(args.srcSys), srcSlot: rd(args.srcSlot),
    srcId: args.srcId, srcCurve: Number(args.srcCurve || 107),
    tgtSys: rd(args.tgtSys), tgtSlot: rd(args.tgtSlot),
    tgtId: args.tgtId, tgtCurve: Number(args.tgtCurve || 77),
    srcCharIdx: Number(args.srcChar || 0), tgtCharIdx: Number(args.tgtChar || 0),
  });
  const outDir = args.outDir || '.';
  fs.mkdirSync(outDir, { recursive: true });
  const slotOut = path.join(outDir, args.slotName || 'data001Slot.bin');
  const sysOut = path.join(outDir, 'data00-1.bin');
  fs.writeFileSync(slotOut, slot);
  fs.writeFileSync(sysOut, sys);
  console.log(JSON.stringify(report, null, 2));
  console.log('slot ->', slotOut, md5(slot));
  console.log('sys  ->', sysOut, md5(sys));
}

module.exports = { migrateSteam2Steam };

if (require.main === module) {
  try { main(process.argv.slice(2)); } catch (e) { console.error('ERROR:', e.message); process.exit(1); }
}
