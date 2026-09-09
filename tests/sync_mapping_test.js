#!/usr/bin/env node
// Hunter-entry sync mapping test: verifies sync_hunter_entry copies the requested
// SOURCE hunter entry into the requested TARGET hunter entry (cross-slot), leaves the
// other target entries untouched, and preserves the target's character GUID.
//
// Run: node tests/sync_mapping_test.js   (uses web/e2e/fixtures)
'use strict';
const fs = require('fs');
const path = require('path');
const wasm = require(path.join(__dirname, '..', 'wasm', 'pkgnode', 'mhsave_wasm.js'));

const U8 = b => new Uint8Array(b);
const FIX = path.join(__dirname, '..', 'web', 'e2e', 'fixtures');
const pcSys = fs.readFileSync(path.join(FIX, 'pc_data00-1.bin'));
const swSys = fs.readFileSync(path.join(FIX, 'switch_data00-1.bin'));
const id = 76561198180024509n;
const curve = 107;

const names = b => wasm.hunter_names(U8(b), id);
function guids(buf) {
  const p = Buffer.from(wasm.decrypted_bytes(U8(buf), id));
  const pat = Buffer.from('cdc00fe4', 'hex');
  const out = [];
  let i = p.indexOf(pat);
  while (i >= 0) {
    const d = (i + 12 + 15) & ~15;
    out.push(p.slice(d, d + 16).toString('hex'));
    i = p.indexOf(pat, i + 1);
  }
  return out;
}

let fail = 0;
const check = (label, cond, extra) => {
  console.log((cond ? '  PASS  ' : '  FAIL  ') + label + (extra ? '  ' + extra : ''));
  if (!cond) fail = 1;
};

const pcNames = names(pcSys);
const swNames = names(swSys);
check('fixture baseline (pc)', JSON.stringify(pcNames) === JSON.stringify(['野口衣織', 'Enix', '(仮)名無し']), JSON.stringify(pcNames));
check('fixture baseline (switch)', JSON.stringify(swNames) === JSON.stringify(['Enix', '(仮)名無し', '(仮)名無し']), JSON.stringify(swNames));

// cross-slot A: switch entry 0 -> target entry 1 (default web layout: switch char in slot 2)
{
  const r = wasm.sync_hunter_entry(U8(pcSys), U8(swSys), id, curve, 1, 0);
  const n = names(r.sys);
  check('tgt1<-src0: entry1 takes source name', n[1] === swNames[0], JSON.stringify(n));
  check('tgt1<-src0: entry0 untouched', n[0] === pcNames[0]);
  check('tgt1<-src0: entry2 untouched', n[2] === pcNames[2]);
  check('tgt1<-src0: target GUID preserved', JSON.stringify(guids(r.sys)) === JSON.stringify(guids(pcSys)));
}

// cross-slot B: switch entry 1 -> target entry 0 (source index != 0, target index != source)
{
  const r = wasm.sync_hunter_entry(U8(pcSys), U8(swSys), id, curve, 0, 1);
  const n = names(r.sys);
  check('tgt0<-src1: entry0 takes source name', n[0] === swNames[1], JSON.stringify(n));
  check('tgt0<-src1: source index was honoured (not Enix)', n[0] !== swNames[0]);
  check('tgt0<-src1: entry1 untouched', n[1] === pcNames[1]);
  check('tgt0<-src1: target GUID preserved', JSON.stringify(guids(r.sys)) === JSON.stringify(guids(pcSys)));
}

// same-index sanity: switch entry 2 -> target entry 2
{
  const r = wasm.sync_hunter_entry(U8(pcSys), U8(swSys), id, curve, 2, 2);
  const n = names(r.sys);
  check('tgt2<-src2: entry2 takes source name', n[2] === swNames[2], JSON.stringify(n));
  check('tgt2<-src2: entries 0/1 untouched', n[0] === pcNames[0] && n[1] === pcNames[1]);
}

console.log(fail ? '\nFAILED' : '\nALL PASS');
process.exit(fail);
