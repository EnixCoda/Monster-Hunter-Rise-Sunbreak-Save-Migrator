import init, { slot_rank, slot_names, slot_times, hunter_names, validate_switch, migrate, sync_hunter_entry } from './wasm/mhsave_wasm.js';

let ready = false;
async function ensure() { if (!ready) { await init(); ready = true; } }

self.onmessage = async (e) => {
  const { id, type, data, sys, template, steamid, curve, srcSys, tgtIdx, srcIdx } = e.data;
  try {
    await ensure();
    let res;
    if (type === 'analyze') {
      const rank = slot_rank(data, steamid);
      res = { rank, ok: !!rank && rank[0] != null };
    } else if (type === 'sys') {
      res = { names: slot_names(data, steamid) };
    } else if (type === 'hunter') {
      res = { names: hunter_names(data, steamid) };
    } else if (type === 'times') {
      res = { times: slot_times(data, steamid) };
    } else if (type === 'validate') {
      res = { ok: validate_switch(data, steamid).startsWith('ok') };
    } else if (type === 'migrate') {
      const r = migrate(data, template, sys, steamid, curve);
      res = { slot: r.slot, sys: r.sys, copied: r.copied };
    } else if (type === 'sync') {
      const r = sync_hunter_entry(sys, srcSys, steamid, curve, tgtIdx, srcIdx);
      res = { sys: r.sys, replaced: r.replaced };
    }
    self.postMessage({ id, ok: true, res });
  } catch (err) {
    self.postMessage({ id, ok: false, error: String(err) });
  }
};
