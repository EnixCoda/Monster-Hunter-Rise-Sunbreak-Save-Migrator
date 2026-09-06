import init, { validate_switch, validate_steam, migrate, detect_curve, slot_rank, slot_names, slot_times, hunter_names } from './wasm/mhsave_wasm.js';
let ready = false;
export async function ensureWasm() {
  if (!ready) { await init(); ready = true; }
  return { validate_switch, validate_steam, migrate, detect_curve, slot_rank, slot_names, slot_times };
}

// ---- Web Worker offload (keeps the UI responsive during heavy decrypts) ----
let worker = null;
let seq = 0;
const pending = new Map();
function getWorker() {
  if (!worker) {
    worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
    worker.onmessage = (e) => {
      const p = pending.get(e.data.id);
      if (!p) return;
      pending.delete(e.data.id);
      e.data.ok ? p.resolve(e.data.res) : p.reject(e.data.error);
    };
  }
  return worker;
}
function req(payload) {
  return new Promise((resolve, reject) => {
    const id = ++seq;
    pending.set(id, { resolve, reject });
    getWorker().postMessage({ id, ...payload });
  });
}
const workerAnalyze = (data, steamid) => req({ type: 'analyze', data, steamid: BigInt(steamid) });
function withTimeout(promise, ms) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('worker timeout')), ms);
    promise.then((v) => { clearTimeout(timer); resolve(v); }, (e) => { clearTimeout(timer); reject(e); });
  });
}
// analyze: prefer Worker; fall back to the main thread if the worker is slow/broken.
export async function analyze(data, steamid) {
  const s = BigInt(steamid);
  try {
    return await withTimeout(workerAnalyze(data, steamid), 6000);
  } catch (e) {
    try {
      const rank = slot_rank(data, s);
      return { rank, ok: !!rank && rank[0] != null };
    } catch (e2) {
      return { rank: null, ok: false };
    }
  }
}
export async function sysNames(data, steamid) {
  try {
    const r = await withTimeout(req({ type: 'sys', data, steamid: BigInt(steamid) }), 6000);
    return r && Array.isArray(r.names) ? r.names : r;
  } catch (e) {
    try { return slot_names(data, BigInt(steamid)); } catch (e2) { return null; }
  }
}
export async function hunterNames(data, steamid) {
  try {
    const r = await withTimeout(req({ type: 'hunter', data, steamid: BigInt(steamid) }), 6000);
    return r && Array.isArray(r.names) ? r.names : r;
  } catch (e) {
    try { return hunter_names(data, BigInt(steamid)); } catch (e2) { return null; }
  }
}
export async function sysTimes(data, steamid) {
  try {
    const r = await withTimeout(req({ type: 'times', data, steamid: BigInt(steamid) }), 6000);
    return r && Array.isArray(r.times) ? r.times : r;
  } catch (e) {
    try { return slot_times(data, BigInt(steamid)); } catch (e2) { return null; }
  }
}
export const migrateOff = (data, template, sys, steamid, curve) => req({ type: 'migrate', data, template, sys, steamid: BigInt(steamid), curve });
