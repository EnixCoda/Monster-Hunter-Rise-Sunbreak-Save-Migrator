<script>
  import { onMount } from 'svelte';
  import { fly, scale } from 'svelte/transition';
  import { FolderOpen, CheckCircle2, ShieldCheck, AlertTriangle, Loader2, Sparkles, ArrowUp, ArrowDown, History, RotateCcw, UserRound } from 'lucide-svelte';
  import { ensureWasm, analyze, sysNames, hunterNames, sysTimes, migrateOff, syncHunter } from './wasm.js';
  import { locale, setLocale, initLocale, fmt } from './lang.js';

  let wasm = null, wasmError = '';
  let busy = false, migrating = false;
  let steamid = '', curve = 107, curveAuto = false;

  let swDir = null, swSlots = [];       // switch slots found (each: name, size, valid, chosen)
  let pcDir = null, pcDirName = '', pcSlots = [];  // pc slots found (each: n, name, status, keep)
  let layout = [];                       // final 3 slots: {slots: [...], indexes}
  let layoutDirty = false;               // becomes true once the user manually reorders
  let pStatus = '', pState = '', sStatus = '', sState = '';
  let result = null, phase = '', progress = 0;
  let backups = [];
  let accCandidates = [];
  let fsSupported = true;
  let confirmOpen = false, confirmSummary = '';
  let busyScan = false, scanPhase = '';
  let loadingBackups = false;
  let restoring = false, restorePhase = '';
  let showBackups = false;
  let lastMatches = [];
  let pendingWasm = false;
  let wiz = 0;

  const MAX = 3;
  const LANG_OPTIONS = [['en', '🌐 English'], ['zh', '🇨🇳 中文'], ['ja', '🇯🇵 日本語']];
  $: t = (k, vars) => fmt(k, vars, $locale);

  onMount(async () => {
    try {
      initLocale();
      fsSupported = !!window.showDirectoryPicker;
      wasm = await ensureWasm();
      const saved = localStorage.getItem('mhsave_steamid');
      if (saved) { steamid = saved; }
      if (swDir) await rescanSwitch();
      if (pcDir) await rescanPc();
    } catch (e) { wasmError = String(e); }
  });
  $: if (wasm && pendingWasm) { pendingWasm = false; if (swDir) rescanSwitch(); if (pcDir) rescanPc(); }
  $: if (steamid) localStorage.setItem('mhsave_steamid', steamid);

  const chosenCount = () => swSlots.filter(x=>x.chosen).length + pcSlots.filter(x=>x.keep).length;
  const canSelectMore = () => chosenCount() < MAX;

  async function rescanSwitch() { if (swDir) { try { let o=[]; for await (const [name,h] of swDir.entries()) { if (name.match(/^data0+\d+Slot\.bin$/)) { const b=await read(h); let v=false; if (wasm&&steamid){try{const a=await analyze(b, steamid); v=a.ok;}catch(e){}} o.push({name,size:b.length,valid:v,chosen:v,bytes:b}); } } o.sort((a,b)=>a.name.localeCompare(b.name)); swSlots=o; rebuildLayout(); } catch(e){} } }
  async function rescanPc() { if (pcDir) { try { let o=[]; for (let i=1;i<=3;i++){ const name=`data00${i}Slot.bin`; let status='empty'; let hr=null; try { const fh=await pcDir.getFileHandle(name); const b=await read(fh); let v=false; if (wasm&&steamid){try{const a=await analyze(b, steamid); v=a.ok; if (a.rank && a.rank[0]!=null) hr=a.rank[0];}catch(e){}} status=v?'valid':(b.length?'unreadable':'empty'); } catch(e){status='empty';} o.push({n:i,name,status,keep:status==='valid',hr}); } pcSlots=o; rebuildLayout(); } catch(e){} } }

  async function rescanPcFull() {
    if (!pcDir) return;
    const o = [];
    for (let i = 1; i <= 3; i++) {
      const name = `data00${i}Slot.bin`;
      let status = 'empty'; let hr = null;
      try {
        const fh = await pcDir.getFileHandle(name);
        const b = await read(fh);
        let ok = false;
        // sync validation here (restore shows a blocking overlay; avoids a worker hang)
        if (wasm && steamid) { try { ok = wasm.validate_switch(b, BigInt(steamid)).startsWith('ok'); } catch (e) {} }
        if (ok && wasm) { try { const r = wasm.slot_rank(b, BigInt(steamid)); if (r && r[0] != null) hr = r[0]; } catch (e) {} }
        status = ok ? 'valid' : (b.length ? 'unreadable' : 'empty');
      } catch (e) { status = 'empty'; }
      o.push({ n: i, name, status, keep: status === 'valid', hr });
    }
    pcSlots = o;
    rebuildLayout();
    // refresh names in the background so it never blocks the restore (worker can be slow)
    (async () => {
      try {
        const sysB = await read(await pcDir.getFileHandle('data00-1.bin'));
        if (!sysB || !sysB.length) return;
        const nn = await hunterNames(sysB, steamid || '1');
        if (!Array.isArray(nn) || !nn.length) return;
        for (const s of pcSlots) {
          const m = s.name.match(/data0+(\d+)Slot\.bin/);
          const idx = m ? parseInt(m[1], 10) - 1 : -1;
          if (idx < 0 || idx >= nn.length) continue;
          s.charName = (typeof nn[idx] === 'string' && nn[idx] && !isPh(nn[idx])) ? nn[idx] : null;
        }
        pcSlots = pcSlots.slice();
      } catch (e) {}
    })();
  }

  const read = async (h) => new Uint8Array(await (await h.getFile()).arrayBuffer());
  const writeFile = async (dir, name, bytes) => { const fh = await dir.getFileHandle(name, { create: true }); const w = await fh.createWritable(); await w.write(bytes); await w.close(); };
  const sleep = (ms) => new Promise(r => setTimeout(r, ms));

  async function createBackup(dir = pcDir, label = '') {
    const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    const name = 'backup_' + ts + (label ? `_${label}` : '');
    const backupDir = await dir.getDirectoryHandle(name, { create: true });
    for await (const [n, h] of dir.entries()) {
      if (n.startsWith('backup_')) continue;
      try { await writeFile(backupDir, n, await read(h)); } catch (e) {}
    }
    await refreshBackups();
    return name;
  }

  async function refreshBackups() {
    if (!pcDir) { backups = []; loadingBackups = false; return; }
    loadingBackups = true;
    const list = [];
    try {
      for await (const [n, dir] of pcDir.entries()) { if (!/^backup_/.test(n)) continue;
        const files = [];
        let total = 0;
        try {
          for await (const [fn, fh] of dir.entries()) {
            try {
              const f = await fh.getFile();
              const size = f.size;
              files.push({ name: fn, size });
              total += size;
            } catch (e) {}
          }
        } catch (e) {}
        list.push({ name: n, dir, files, total });
      }
      list.sort((a, b) => b.name.localeCompare(a.name));
    } catch (e) {}
    backups = list;
    loadingBackups = false;
  }

  async function loadBackupNames() {
    for (const b of backups) {
      if (b.names) continue;
      try { const sysB = await read(await b.dir.getFileHandle('data00-1.bin')); const nn = await hunterNames(sysB, steamid || '1'); if (nn) { b.names = nn.filter(x => typeof x === 'string' && x).slice(0, 3).join(' · '); backups = backups.slice(); } } catch (e) {}
    }
  }
  async function restoreBackup(b) {
    if (restoring || busy) return;
    if (!confirm(t('backups.confirmRestore', { name: b.name }))) return;
    restoring = true; restorePhase = t('run.restorePhase');
    try {
      const pre = await createBackup(pcDir, 'prerestore');
      for (const f of b.files) {
        restorePhase = t('run.restorePhase');
        const bytes = await read(await b.dir.getFileHandle(f.name));
        await writeFile(pcDir, f.name, bytes);
      }
      await rescanPcFull();
      await refreshBackups();
      result = null;
      pStatus = t('backups.restored', { name: b.name, pre }); pState = 'ok';
    } catch (e) { pStatus = t('backups.restoreFailed', { e: String(e || '…') }); pState = 'err'; }
    finally { restoring = false; restorePhase = ''; }
  }

  async function deleteBackup(b) {
    if (restoring || busy) return;
    if (!confirm(t('backups.confirmDelete', { name: b.name }))) return;
    try { await pcDir.removeEntry(b.name, { recursive: true }); } catch (e) { pStatus = t('backups.deleteFailed', { e }); pState = 'err'; }
    await refreshBackups();
  }

  function rebuildLayout() {
    const chosen = new Map();
    for (const s of swSlots) if (s.chosen) chosen.set('s:' + s.name, { kind: 'switch', src: s });
    for (const s of pcSlots) if (s.keep) chosen.set('k:' + s.name, { kind: 'keep', src: s });
    let next = [];
    for (const item of layout) {
      const id = (item.kind === 'switch' ? 's:' : 'k:') + item.src.name;
      const c = chosen.get(id);
      if (c) { next.push(c); chosen.delete(id); }
    }
    // Default: keep existing Steam characters in their slots first, then append Switch chars
    // (minimises changes to the Steam save).
    for (const s of pcSlots) { const c = chosen.get('k:' + s.name); if (c) { next.push(c); chosen.delete('k:' + s.name); } }
    for (const s of swSlots) { const c = chosen.get('s:' + s.name); if (c) { next.push(c); chosen.delete('s:' + s.name); } }
    // Default: keep existing Steam characters first (minimise Steam-save changes);
    // stop auto-sorting once the user manually reorders.
    if (!layoutDirty) {
      const keeps = next.filter(x => x.kind === 'keep');
      const switches = next.filter(x => x.kind === 'switch');
      next = keeps.concat(switches);
    }
    layout = next.slice(0, MAX);
  }

  function move(i, dir) {
    const j = i + dir;
    if (j < 0 || j >= layout.length) return;
    layoutDirty = true;
    layout = layout.slice();
    const tmp = layout[i]; layout[i] = layout[j]; layout[j] = tmp;
  }

  function isPh(n) { return !n || n === '(仮)名無し' || n === '（仮）名無し'; }
  function slotDisplay(s) { return s.charName || t('slot.pos', { n: s.n || 1 }); }
  function backupLabel(b) {
    const m = b.name.match(/^backup_(\d{4}-\d{2}-\d{2})T(\d{2})-(\d{2})-(\d{2})(?:_(.*))?$/);
    if (!m) return { title: b.name, tag: '' };
    return { title: `${m[1]} ${m[2]}:${m[3]}`, tag: m[4] === 'prerestore' ? 'prerestore' : '' };
  }

  async function pickSwitch() {
    if (!window.showDirectoryPicker) { sStatus = t('pick.needChrome'); sState = 'err'; return; }
    if (!wasm) pendingWasm = true;
    try {
      swDir = await window.showDirectoryPicker();
      let found = [];
      for await (const [name, h] of swDir.entries()) {
        if (name.match(/^data0+\d+Slot\.bin$/)) {
          const bytes = await read(h);
          let valid = false;
          if (wasm) { try { valid = wasm.validate_switch(bytes, BigInt(steamid || '1')).startsWith('ok'); } catch (e) {} }
          let hr = null; if (wasm) { try { const r = wasm.slot_rank(bytes, BigInt(steamid || '1')); if (r && r[0] != null) hr = r[0]; } catch (e) {} }
          found.push({ name, size: bytes.length, valid, chosen: false, bytes, hr, n: (found.length + 1) });
        }
      }
      found.sort((a, b) => a.name.localeCompare(b.name));
      swSlots = found;
      // default: choose valid ones
      for (const s of swSlots) s.chosen = s.valid;
      rebuildLayout();
      if (!found.length) { sStatus = t('s1.noSlots'); sState = 'err'; return; }
      await applySysNames(swSlots, async () => { try { return await read(await swDir.getFileHandle('data00-1.bin')); } catch (e) { return null; } });
      swSlots = swSlots.slice();
      sStatus = '';
    } catch (e) { sStatus = e && e.name === 'AbortError' ? t('pick.cancelled') : t('pick.readErr', { e: e || '…' }); sState = 'err'; }
  }

  async function pickPc() {
    if (!window.showDirectoryPicker) { pStatus = t('pick.needChrome'); pState = 'err'; return; }
    if (!wasm) pendingWasm = true;
    try {
      const root = await window.showDirectoryPicker();
      let found = null, acc = '';
      // The user may have picked win64_save directly (it contains data00-1.bin) — accept it.
      let direct = false;
      try { await root.getFileHandle('data00-1.bin'); direct = true; } catch (e) {}
      if (direct) {
        if (!steamid) {
          pStatus = t('s2.errNeedId'); pState = 'err'; return;
        }
        found = root;
      } else {
        // find <accountid>/1446780/remote/win64_save — there may be several accounts
        accCandidates = [];
        const matches = [];
        for await (const [accName, accHandle] of root.entries()) {
          if (!/^\d+$/.test(accName)) continue;
          let win64 = null, chars = 0;
          try {
            const remote = await accHandle.getDirectoryHandle('1446780', { create: false });
            const r2 = await remote.getDirectoryHandle('remote', { create: false });
            win64 = await r2.getDirectoryHandle('win64_save', { create: false });
            for (let i = 1; i <= 3; i++) { try { const fh = await win64.getFileHandle(`data00${i}Slot.bin`); const b = await read(fh); if (b.length) chars++; } catch (e) {} }
          } catch (e) {}
          matches.push({ accName, win64, chars, hasSave: !!win64 });
        }
        if (!matches.length) { pStatus = t('s2.errNoSave'); pState = 'err'; return; }
        lastMatches = matches;
        const hasSaves = matches.filter(m => m.hasSave);
        const remembered = localStorage.getItem('mhsave_accountid');
        const chosen = hasSaves.find(m => m.accName === remembered) || (hasSaves.length === 1 ? hasSaves[0] : null);
        if (chosen) {
          await useAccountPc(chosen.accName, chosen.win64);
        } else {
          accCandidates = matches;
          pStatus = t('s2.multi', { n: matches.length, s: hasSaves.length }); pState = 'ok';
          return;
        }
        return;
      }
      if (!found) { pStatus = t('s2.errNoSave'); pState = 'err'; return; }
      pcDir = found; pcDirName = found.name;
      if (acc) {
        const accountid = BigInt(acc);
        const id64 = 0x0110000100000000n + accountid;
        steamid = id64.toString();
        localStorage.setItem('mhsave_steamid', steamid);
      }
      let foundPc = [];
      busyScan = true;
      let done = 0;
      scanPhase = t('scan.reading', { n: 0, total: 3 });
      const jobs = [1, 2, 3].map(async (i) => {
        const name = `data00${i}Slot.bin`;
        let status = 'empty'; let hr = null, mr = null;
        try {
          const fh = await pcDir.getFileHandle(name);
          const bytes = await read(fh);
          let ok = false;
          try { const a = await analyze(bytes, steamid || '1'); ok = a.ok; if (a.rank && a.rank[0] != null) { hr = a.rank[0]; if (a.rank[1] != null) mr = a.rank[1]; } } catch (e) {}
          status = ok ? 'valid' : (bytes.length ? 'unreadable' : 'empty');
        } catch (e) { status = 'empty'; }
        done += 1; scanPhase = t('scan.reading', { n: done, total: 3 });
        return { n: i, name, status, keep: status === 'valid', hr, mr };
      });
      foundPc = await Promise.all(jobs);
      try { await applySysNames(foundPc, async () => { try { return await read(await pcDir.getFileHandle('data00-1.bin')); } catch (e) { return null; } }); } catch (e) {}
      busyScan = false; scanPhase = '';
      pcSlots = foundPc;
      rebuildLayout();
      await refreshBackups();
      pStatus = acc
        ? t('s2.foundAccount', { a: acc })
        : t('s2.foundDirect', { id: steamid });
      pState = 'ok';
      } catch (e) { pStatus = e && e.name === 'AbortError' ? t('pick.cancelled') : t('pick.readErr', { e: e || '…' }); pState = 'err'; }
  }

  async function useAccountPc(accName, win64) {
    pcDir = win64; pcDirName = win64.name; accCandidates = [];
    if (!wasm) pendingWasm = true;
    const accountid = BigInt(accName);
    const id64 = 0x0110000100000000n + accountid;
    steamid = id64.toString();
    localStorage.setItem('mhsave_steamid', steamid);
    localStorage.setItem('mhsave_accountid', accName);
    let foundPc = [];
    busyScan = true;
    let done = 0;
    scanPhase = t('scan.reading', { n: 0, total: 3 });
    const jobs = [1, 2, 3].map(async (i) => {
      const name = `data00${i}Slot.bin`;
      let status = 'empty'; let hr = null, mr = null;
      try {
        const fh = await pcDir.getFileHandle(name);
        const bytes = await read(fh);
        let ok = false;
        try { const a = await analyze(bytes, steamid || '1'); ok = a.ok; if (a.rank && a.rank[0] != null) { hr = a.rank[0]; if (a.rank[1] != null) mr = a.rank[1]; } } catch (e) {}
        status = ok ? 'valid' : (bytes.length ? 'unreadable' : 'empty');
      } catch (e) { status = 'empty'; }
      done += 1; scanPhase = t('scan.reading', { n: done, total: 3 });
      return { n: i, name, status, keep: status === 'valid', hr, mr };
    });
    foundPc = await Promise.all(jobs);
    try { await applySysNames(foundPc, async () => { try { return await read(await pcDir.getFileHandle('data00-1.bin')); } catch (e) { return null; } }); } catch (e) {}
    busyScan = false; scanPhase = '';
    pcSlots = foundPc;
    rebuildLayout();
    await refreshBackups();
    pStatus = t('s2.foundAccount', { a: accName }); pState = 'ok';
  }
  function selectAccount(c) { useAccountPc(c.accName, c.win64); }
  const idFromAcc = (a) => (0x0110000100000000n + BigInt(a)).toString();
  async function applySysNames(list, readSys) {
    if (!wasm) return;
    try {
      const sysB = await readSys();
      if (!sysB || !sysB.length) return;
      const nn = await hunterNames(sysB, steamid || '1'); 
      let tt = null; try { tt = await sysTimes(sysB, steamid || '1'); } catch (e) {}
      if (!nn || !nn.length) return;
      for (const s of list) {
        const m = s.name.match(/data0+(\d+)Slot\.bin/);
        const idx = m ? parseInt(m[1], 10) - 1 : -1;
        if (idx < 0 || idx >= nn.length) continue;
        if (typeof nn[idx] === 'string' && nn[idx] && !isPh(nn[idx])) { s.charName = nn[idx]; } else { s.charName = null; if (s.status === 'valid') s.status = 'empty'; if ('keep' in s) s.keep = false; }
      }
    } catch (e) {}
  }
  const wizUnlocked = (n) => (n <= 2 ? true : n <= 3 && !!pcDir);
  function goWiz(n) { if (restoring || busy) return; if (n < 0 || n > 3) return; if (n === 2 && !swSlots.length) return; if (n === 3 && !pcDir) return; wiz = n; if (n === 3) { pStatus = ''; sStatus = ''; } }

  function toggleSwitch(s) { if (!s.chosen && !canSelectMore()) { pStatus = t('msg.maxSlots'); return; } s.chosen = !s.chosen; swSlots = swSlots.slice(); rebuildLayout(); }
  function togglePc(s) { if (!s.keep && !canSelectMore()) { pStatus = t('msg.maxSlots'); return; } s.keep = !s.keep; pcSlots = pcSlots.slice(); rebuildLayout(); }

  async function run() {
    if (chosenCount() > MAX) { result = { error: t('run.wrongCount', { n: chosenCount(), extra: chosenCount() - MAX }) }; return; }
    if (!wasm || !pcDir || !steamid || layout.filter(l => l.kind === 'switch').length === 0) return;
    const removedNames = pcSlots.filter(s => !s.keep && s.status !== 'empty').map(s => slotDisplay(s));
    const slotLine = (l) => l ? (l.kind === 'switch' ? t('run.switchChar', { name: slotDisplay(l.src) }) : t('run.keptChar', { name: slotDisplay(l.src) })) : t('run.empty');
    confirmSummary = [
      '',
      t('run.backupFirst'),
      '',
      t('run.finalSlots'),
      ...layout.map((l, i) => `  ${t('s3.slot', { i: i + 1 })}: ${slotLine(l)}`),
      '',
      removedNames.length ? t('run.removed', { names: removedNames.join(', ') }) : null,
      t('run.steamId', { id: steamid }),
      '',
      t('run.cloud'),
    ].filter(Boolean).join('\n');
    confirmOpen = true;
  }

  function restoreNow() {
    const b = backups.find(x => x.name === (result && result.backup));
    if (b) restoreBackup(b);
  }
  function rePickAccount() {
    if (!lastMatches.length) return;
    pcDir = null; pcDirName = ''; pcSlots = []; backups = []; result = null;
    accCandidates = lastMatches;
    rebuildLayout();
    pStatus = t('s2.multi', { n: lastMatches.length }); pState = 'ok';
  }

  async function doRun() {
    confirmOpen = false;
    busy = true; migrating = true; result = null; progress = 6; phase = t('msg.reading');
    try {
      // Snapshot ALL current files in memory BEFORE any write — keep-sources and templates must be originals.
      const original = {};
      for (let i = 1; i <= 3; i++) { const n = `data00${i}Slot.bin`; try { original[n] = await read(await pcDir.getFileHandle(n)); } catch (e) {} }
      let sys = []; try { sys = await read(await pcDir.getFileHandle('data00-1.bin')); } catch (e) {}
      // Switch sys — source of the title-screen character entries (name/HR/MR/buddies/appearance).
      let swSys = []; try { swSys = await read(await swDir.getFileHandle('data00-1.bin')); } catch (e) {}
      // Back up the whole original save first (inside win64_save, before touching anything).
      const backupName = await createBackup(pcDir);

      let count = 0, written = []; const checks = [];
      for (let i = 0; i < MAX; i++) {
        const targetName = `data00${i + 1}Slot.bin`;
        const item = layout[i];
        if (item && item.kind === 'switch') {
          let template = (original[targetName] && original[targetName].length) ? original[targetName] : null;
          if (!template) for (const l of layout) if (l.kind === 'keep' && original[l.src.name] && original[l.src.name].length) { template = original[l.src.name]; break; }
          if (!template) { result = { error: t('run.slotNeeds', { i: i + 1, backup: backupName }) }; busy = false; setTimeout(() => migrating = false, 500); return; }
          phase = t('run.migratePhase', { i: i + 1, name: slotDisplay(item.src) }); progress = 25 + (i * 18);
          await sleep(150);
          const r = await migrateOff(item.src.bytes, template, sys, steamid, curve);
          if (r.copied === 0) {
            localStorage.removeItem('mhsave_accountid');
            result = { error: t('run.diffAcc', { id: steamid, backup: backupName }), diffAcc: true };
            busy = false; setTimeout(() => migrating = false, 500); return;
          }
          sys = r.sys;
          // Also sync the title-screen entry (name/HR/MR/play time/buddies/outfit colours
          // + gender/face/hair/voice appearance class) from the matching Switch sys entry.
          try {
            const sm = item.src.name.match(/data0+(\d+)Slot\.bin/);
            const srcIdx = sm ? parseInt(sm[1], 10) - 1 : -1;
            if (swSys && swSys.length && srcIdx >= 0) {
              const sr = await syncHunter(sys, swSys, steamid, curve, i, srcIdx);
              if (sr && sr.sys) sys = sr.sys;
            }
          } catch (e) {}
          const out = new Uint8Array(r.slot);
          await writeFile(pcDir, targetName, out);
          written.push(t('res.wWrittenSwitch', { target: targetName, name: slotDisplay(item.src) }));
          let ok = false, vhr = null;
          try { const a = await analyze(out, steamid); ok = a.ok; if (a.rank && a.rank[0] != null) vhr = a.rank[0]; } catch (e) {}
          checks.push({ i: i + 1, ok, hr: vhr });
          count += r.copied;
        } else if (item && item.kind === 'keep') {
          const saved = original[item.src.name];
          if (!saved || !saved.length) { result = { error: t('run.couldNotRead', { name: item.src.name, backup: backupName }) }; busy = false; setTimeout(() => migrating = false, 500); return; }
          await writeFile(pcDir, targetName, saved);
          written.push(t('res.wWrittenKept', { target: targetName, name: slotDisplay(item.src) }));
          checks.push({ i: i + 1, ok: true, hr: item.hr });
        } else {
          try { await pcDir.removeEntry(targetName); } catch (e) {}
          written.push(t('res.wWrittenEmpty', { target: targetName }));
          checks.push({ i: i + 1, ok: true });
        }
      }
      if (sys.length) await writeFile(pcDir, 'data00-1.bin', sys);
      progress = 100; phase = t('msg.done');
      await refreshBackups();
      result = { copied: count, backup: backupName, folder: pcDirName, written, checks };
    } catch (e) { result = { error: t('res.errGeneric', { e: String(e || '…') }) }; }
    finally { busy = false; setTimeout(() => { migrating = false; phase = ''; }, 900); }
  }
</script>

<svelte:window on:keydown={(e) => { if (confirmOpen && e.key === 'Escape') confirmOpen = false; }} />

<div class="bg-scene"></div>
<div class="orb orb-a"></div>
<div class="orb orb-b"></div>

<div class="relative min-h-screen">
  <div class="mx-auto max-w-2xl px-6 py-12">
    <header class="mb-8 text-center">
      <h1 class="font-display title-glow text-4xl font-bold tracking-[.2em] gold-grad-text">{t('brand.game')}</h1>
      <p class="mt-3 font-display title-glow text-lg font-semibold tracking-[.34em] text-amber-200/90">{t('brand.title')}</p>
      <div class="mx-auto my-4 h-px w-44 bg-gradient-to-r from-transparent via-amber-200/40 to-transparent"></div>
      <p class="text-sm text-amber-200/60 flicker">{t('brand.tagline')}</p>
      <p class="mt-1 text-xs text-amber-200/45">{t('brand.items')}</p>
      <div class="mt-5 flex items-center justify-center gap-2 text-xs">
        <button on:click={() => goWiz(0)} class="rounded-full border px-2.5 py-0.5 {wiz === 0 ? 'border-amber-300 bg-amber-500/15 text-amber-200' : 'border-line text-zinc-500'}">0 · {t('rail.prep')}</button>
        <span class="text-zinc-600">→</span>
        <button on:click={() => goWiz(1)} class="rounded-full border px-2.5 py-0.5 {swSlots.length && wiz >= 1 ? 'border-amber-300 bg-amber-500/15 text-amber-200' : 'border-line text-zinc-500'}">1 · {t('rail.switch')}</button>
        <span class="text-zinc-600">→</span>
        <button on:click={() => goWiz(2)} class="rounded-full border px-2.5 py-0.5 {pcSlots.length && wiz >= 2 ? 'border-amber-300 bg-amber-500/15 text-amber-200' : 'border-line text-zinc-500'}">2 · {t('rail.steam')}</button>
        <span class="text-zinc-600">→</span>
        <button on:click={() => goWiz(3)} class="rounded-full border px-2.5 py-0.5 {layout.length && wiz >= 3 ? 'border-amber-300 bg-amber-500/15 text-amber-200' : 'border-line text-zinc-500'}">3 · {t('rail.layout')}</button>
      </div>
    </header>

    {#if wasmError}<div class="mb-6 rounded-2xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-300"><b>{t('wasm.failed')}</b> {wasmError}</div>{/if}
    {#if !fsSupported}<div class="mb-6 flex items-start gap-2 rounded-2xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm text-amber-200/90"><AlertTriangle size={16} class="mt-0.5 shrink-0" /> <span>{t('fs.warn')}</span></div>{/if}

    <ol class="space-y-6">
      <li class="panel p-6" in:fly={{ y: 24, duration: 500 }} hidden={wiz !== 0}>
        <div class="flex items-center gap-3"><span class="medal">0</span><h2 class="font-display text-lg font-semibold tracking-wide">{t('s0.heading')}</h2></div>
        <div class="mt-3 text-sm text-zinc-400">{@html t('s0.desc')}</div>
      </li>
      <li class="panel p-6" in:fly={{ y: 24, duration: 500 }} hidden={wiz !== 1}>
        <div class="flex items-center gap-3"><span class="medal">1</span><h2 class="font-display text-lg font-semibold tracking-wide">{t('s1.heading')}</h2></div>
        <p class="mt-2 text-sm text-zinc-400">{t('s1.desc')}</p>
        <button on:click={pickSwitch} class="btn btn-gold mt-4"><FolderOpen size={15} /> {t('s1.pick')}</button>
        {#if swSlots.length}
          <div class="mt-4 grid gap-2">
            {#each swSlots.filter(s => s.bytes && s.bytes.length > 0) as s (s.name)}
              <div class="flex items-center gap-3 rounded-xl border border-line bg-ink2/40 p-3 text-sm">
                <UserRound size={15} class="shrink-0 text-amber-200/70" />
                <span class="text-zinc-200">{slotDisplay(s)}</span>
                {#if s.hr != null}<span class="text-xs text-zinc-500">HR {s.hr}</span>{/if}
                {#if s.lastSave}<span class="text-xs text-zinc-600">{s.lastSave}</span>{/if}
                <span class="ml-auto text-xs text-zinc-400">{s.valid?t('s1.valid'):t('s1.unreadable')}</span>
              </div>
            {/each}
          </div>
        {/if}
      </li>

      <li class="panel p-6" in:fly={{ y: 24, delay: 100, duration: 500 }} hidden={wiz !== 2}>
        <div class="flex items-center gap-3"><span class="medal">2</span><h2 class="font-display text-lg font-semibold tracking-wide">{t('s2.heading')}</h2></div>
        <p class="mt-2 text-sm text-zinc-400">{@html t('s2.desc')}</p>
        <details class="mt-3 text-xs text-zinc-500">
          <summary class="cursor-pointer hover:text-zinc-300">{t('s2.where.title')}</summary>
          <p class="mt-2 text-zinc-400">{@html t('s2.where.body')}</p>
        </details>
        <button on:click={pickPc} class="btn btn-gold mt-4"><FolderOpen size={15} /> {t('s2.pick')}</button>
        {#if accCandidates.length && !pcDir}
          <div class="mt-4 grid gap-2">
            {#each accCandidates as c (c.accName)}
              <button on:click={() => c.hasSave && selectAccount(c)} disabled={!c.hasSave} class="flex items-center gap-3 rounded-xl border border-line bg-ink2/40 p-3 text-left text-sm transition {c.hasSave ? 'hover:border-amber-500/50' : 'opacity-60'}">
                <span class="grid h-5 w-5 shrink-0 place-items-center rounded border border-zinc-600"><UserRound size={12} /></span>
                <span class="min-w-0"><span class="text-xs text-zinc-500">{t('s2.account')}</span><br /><span class="font-mono text-zinc-200">{c.accName}</span>{#if c.hasSave}<span class="ml-2 text-xs text-zinc-500">{t('s2.chars', { n: c.chars })}</span>{:else}<span class="ml-2 text-xs text-red-300/80">{t('s2.noSave')}</span>{/if}</span>
                <span class="ml-auto shrink-0 text-right text-xs text-zinc-400">{t('s2.steamid')}<br /><span class="font-mono text-amber-200/90">{idFromAcc(c.accName)}</span></span>
              </button>
            {/each}
          </div>
        {/if}
        {#if pcSlots.length}
          <div class="mt-4 grid gap-2">
            {#each pcSlots.filter(s => s.status !== 'empty') as s (s.name)}
              <div class="flex items-center gap-3 rounded-xl border border-line bg-ink2/40 p-3 text-sm">
                <UserRound size={15} class="shrink-0 text-amber-200/70" />
                <span class="text-zinc-200">{slotDisplay(s)}</span>
                {#if s.hr != null}<span class="text-xs text-zinc-500">HR {s.hr}</span>{/if}
                {#if s.lastSave}<span class="text-xs text-zinc-600">{s.lastSave}</span>{/if}
                <span class="ml-auto text-xs text-zinc-400">{s.status === 'valid' ? t('s2.character') : s.status === 'unreadable' ? t('s1.unreadable') : t('s2.empty')}</span>
              </div>
            {/each}
          </div>
        {/if}
              </li>

      <li class="panel p-6" in:fly={{ y: 24, delay: 200, duration: 500 }} hidden={wiz !== 3}>
        <div class="flex items-center gap-3"><span class="medal">3</span><h2 class="font-display text-lg font-semibold tracking-wide">{t('s3.heading')}</h2></div>
        <p class="mt-2 text-sm text-zinc-400">{t('s3.desc')}</p>
        {#if swSlots.length && pcSlots.length}
          <div class="mt-4 grid gap-4 sm:grid-cols-2">
            <div>
              <p class="mb-1.5 text-xs font-semibold text-zinc-400">{t('s3.pickSwitch')}</p>
              <div class="grid gap-1.5">
                {#each swSlots.filter(s => s.valid) as s (s.name)}
                  <button on:click={() => toggleSwitch(s)} aria-label={s.name} disabled={!s.valid} class="flex items-center gap-2 rounded-lg border p-2 text-left text-sm {s.chosen?'border-amber-300 bg-amber-500/10 text-amber-200':'border-line text-zinc-400'} {s.valid?'':'opacity-50'}">
                    <span class="grid h-4 w-4 place-items-center rounded border text-[10px]">{s.chosen?'✓':''}</span>
                    <span>{slotDisplay(s)}</span>{#if s.hr != null}<span class="text-xs text-zinc-500">HR {s.hr}</span>{/if}
                  </button>
                {/each}
              </div>
            </div>
            <div>
              <p class="mb-1.5 text-xs font-semibold text-zinc-400">{t('s3.pickKeep')}</p>
              <div class="grid gap-1.5">
                {#each pcSlots.filter(s => s.status !== 'empty') as s (s.name)}
                  <button on:click={() => togglePc(s)} aria-label={s.name} disabled={s.status !== 'valid'} class="flex items-center gap-2 rounded-lg border p-2 text-left text-sm {s.keep?'border-amber-300 bg-amber-500/10 text-amber-200':'border-line text-zinc-400'} {s.status !== 'valid'?'opacity-50':''}">
                    <span class="grid h-4 w-4 place-items-center rounded border text-[10px]">{s.keep?'✓':''}</span>
                    <span>{slotDisplay(s)}</span>{#if s.hr != null}<span class="text-xs text-zinc-500">HR {s.hr}</span>{/if}
                    <span class="ml-auto text-[10px] text-zinc-500">{s.status === 'valid' ? t('s2.character') : s.status === 'unreadable' ? t('s1.unreadable') : t('s2.empty')}</span>
                  </button>
                {/each}
              </div>
            </div>
          </div>
        <div class="mt-4 grid gap-2">
          {#each layout as item, i}
            {#if item}
              <div data-kind={item.kind} class="flex items-center gap-3 rounded-xl border border-line bg-ink2/40 p-3 text-sm">
                <span class="rounded px-1.5 py-0.5 text-[10px] font-semibold {item.kind === 'switch' ? 'bg-amber-500/15 text-amber-200' : 'bg-zinc-500/15 text-zinc-300'}">{item.kind === 'switch' ? t('s3.srcSwitch') : t('s3.srcSteam')}</span>
                <span class="{item.kind === 'switch' ? 'text-amber-200' : 'text-zinc-300'}">{slotDisplay(item.src)}</span>
                {#if item.src.hr != null}<span class="text-xs text-zinc-500">HR {item.src.hr}</span>{/if}
                {#if item.src.lastSave}<span class="text-xs text-zinc-600">· {item.src.lastSave}</span>{/if}
                <span class="ml-auto flex items-center gap-1">
                  <button on:click={() => move(i, -1)} disabled={i === 0} aria-label={t('s3.moveUp')} title={t('s3.moveUp')} class="rounded border border-line p-1 text-zinc-400 hover:text-amber-200 disabled:opacity-30"><ArrowUp size={14} /></button>
                  <button on:click={() => move(i, 1)} disabled={i === layout.length - 1} aria-label={t('s3.moveDown')} title={t('s3.moveDown')} class="rounded border border-line p-1 text-zinc-400 hover:text-amber-200 disabled:opacity-30"><ArrowDown size={14} /></button>
                </span>
              </div>
            {/if}
          {/each}
        </div>
        {:else}
        <p class="mt-4 text-sm text-zinc-500">{t('s3.wait')}</p>
        {/if}
        {#if pcSlots.length && !pcSlots.some(s => s.status === 'valid') && !pcSlots.some(s => !s.keep && s.status !== 'empty')}
          <div class="mt-4 flex items-start gap-2 rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-200/90">
            <AlertTriangle size={14} class="mt-0.5 shrink-0" />
            <span title={t('why.template')}>{t('s3.noTemplate')}</span>
          </div>
        {/if}
        {#if pcSlots.some(s => !s.keep && s.status !== 'empty')}
          <div class="mt-4 flex items-start gap-2 rounded-xl border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-amber-200/90">
            <AlertTriangle size={14} class="mt-0.5 shrink-0" />
            <span>{@html t('s3.warn', { names: pcSlots.filter(s => !s.keep && s.status !== 'empty').map(s => slotDisplay(s)).join(', ') })}</span>
          </div>
        {/if}
        <button on:click={run} disabled={busy || restoring || !wasm || !pcDir || !steamid || !swSlots.some(s=>s.chosen)} class="btn btn-gold mt-4 text-base">
          {#if busy}<Loader2 size={16} class="spin" /> {t('s3.migrating')}{:else}<Sparkles size={16} /> {t('s3.run')}{/if}
        </button>
        {#if !wasm}<p class="mt-2 text-center text-xs text-zinc-500">{t('s3.hintWasm')}</p>
        {:else if !pcDir}<p class="mt-2 text-center text-xs text-zinc-500">{t('s3.hintPick')}</p>
        {:else if !swSlots.length}<p class="mt-2 text-center text-xs text-zinc-500">{t('s3.hintNoSwitch')}</p>
        {:else if !swSlots.some(s=>s.chosen)}<p class="mt-2 text-center text-xs text-zinc-500">{t('s3.hintPickSwitch')}</p>{/if}
        {#if migrating}
          <div class="mt-3">
            <div class="mb-1 flex justify-between text-xs text-amber-200/80"><span>{phase || t('msg.ready')}</span><span>{Math.round(progress)}%</span></div>
            <div class="progress"><i style={`width:${progress}%`}></i></div>
          </div>
        {/if}
      </li>
    </ol>

    {#if sStatus}<div class="mb-2"><span class="pill {sState==='ok'?'pill-ok':'pill-err'} text-xs">{sStatus}</span></div>{/if}
    {#if pStatus}<div class="mb-4"><span class="pill {pState==='ok'?'pill-ok':'pill-err'} text-xs">{pStatus}</span></div>{/if}
    {#if pcDirName}
    <section class="panel p-6 mb-8">
      <button on:click={() => (showBackups = !showBackups)} class="flex w-full items-center gap-2 text-left text-sm font-semibold text-zinc-300" aria-expanded={showBackups}>
        <History size={14} /> {t('backups.title')} <span class="text-xs font-normal text-zinc-500">({backups.length}) · {t('backups.hint')}</span>
        <span class="ml-auto text-xs text-zinc-500">{showBackups ? '−' : '+'}</span>
      </button>
      {#if showBackups}
      <div class="mt-4 border-t border-line pt-4">
            {#if backups.length}
              <p class="mb-2 text-xs text-zinc-500">{t('backups.legend')}</p>
              <div class="grid gap-2">
                {#each backups as b (b.name)}
                  {@const label = backupLabel(b)}
                  <div class="flex items-center gap-3 rounded-xl border border-line bg-ink2/40 p-3 text-sm">
                    <div class="min-w-0">
                      <div class="flex items-center gap-2">
                        <span class="text-zinc-200">{label.title}</span>
                        {#if label.tag}<span class="rounded bg-amber-500/15 px-1.5 py-0.5 text-[10px] text-amber-200/90">{t('backups.tagPrerestore')}</span>{/if}
                      </div>
                      <div class="mt-0.5 truncate text-xs text-zinc-500">{t('backups.files', { n: b.files.length, kb: (b.total/1024).toFixed(0) })}{#if b.names}<span class="text-amber-200/70"> · 「{b.names}」</span>{/if} <span class="font-mono">{b.name}</span></div>
                    </div>
                    <span class="ml-auto flex shrink-0 items-center gap-2">
                      <button on:click={() => restoreBackup(b)} class="btn btn-gold px-3 py-1.5 text-xs"><RotateCcw size={13} /> {t('backups.restore')}</button>
                      <button on:click={() => deleteBackup(b)} class="rounded px-2 py-1 text-xs text-zinc-500 hover:text-red-300">{t('backups.delete')}</button>
                    </span>
                  </div>
                {/each}
              </div>
            {:else if loadingBackups && !backups.length}
              <p class="text-xs text-amber-200/70"><Loader2 size={12} class="spin inline" /> {t('backups.loading')}</p>
            {:else}
              <p class="text-xs text-zinc-500">{t('backups.none')}</p>
            {/if}
          </div>
        {/if}
    </section>
    {/if}

    <div class="mb-8 flex items-center justify-center gap-2">
      {#if wiz > 0}<button on:click={() => goWiz(wiz - 1)} class="btn btn-ghost text-xs">← {t('wiz.back')}</button>{/if}
      {#if wiz < 3}
        <button on:click={() => goWiz(wiz + 1)} disabled={(wiz === 1 && !swSlots.length) || (wiz === 2 && !pcDir)} class="btn btn-gold text-xs">{t('wiz.next')} →</button>
      {:else}
        <span class="text-xs text-zinc-500">{t('wiz.last')}</span>
      {/if}
    </div>

    <footer class="mb-8 flex items-center justify-center gap-1">
      {#each LANG_OPTIONS as [code, label]}
        <button on:click={() => setLocale(code)} class="rounded-lg border px-3 py-1 text-xs transition {($locale === code) ? 'border-amber-300 bg-amber-500/15 text-amber-200' : 'border-line text-zinc-400 hover:text-zinc-200'}">{label}</button>
      {/each}
    </footer>

    {#if result && !result.error}
      <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4" role="dialog" aria-modal="true">
        <div class="panel w-full max-w-lg p-6">
          <p class="flex items-center gap-2 font-display text-amber-200"><CheckCircle2 size={20} /> {t('res.complete')}</p>
          <p class="mt-1 text-sm italic text-amber-200/60">{t('res.flavor')}</p>
          <ul class="mt-3 space-y-1 text-sm text-zinc-300">
            {#each result.written as w}<li class="pop">{w}</li>{/each}
            <li class="pop" style="animation-delay:.15s">{t('res.backup')} <code class="text-amber-200/90">{result.folder}/{result.backup}</code></li>
            <li class="text-zinc-400">{t('res.next')}</li>
            <li class="pop" style="animation-delay:.25s">{t('res.verify')}：
              {#each result.checks || [] as c}
                <span class="text-xs {c.ok ? 'text-emerald-300' : 'text-red-300'}"> {c.ok ? '✔' : '✖'} {t('s3.slot', { i: c.i })}{#if c.hr != null} · HR {c.hr}{/if}</span>
              {/each}
            </li>
          </ul>
          <div class="mt-4 flex flex-wrap items-center gap-2">
            <button on:click={() => { result = null; }} class="btn btn-ghost text-xs"><CheckCircle2 size={13} /> {t('res.verifyOk')}</button>
            <button on:click={restoreNow} class="btn btn-gold text-xs"><RotateCcw size={13} /> {t('res.restoreNow')}</button>
          </div>
        </div>
      </div>
    {:else if result}
      <div class="result-glow panel mt-6 p-6" transition:scale={{ duration: 300, start: .96 }}>
        <p class="flex items-start gap-2 text-sm text-red-300"><AlertTriangle size={18} class="mt-0.5 shrink-0" /> {result.error}</p>
        {#if result.diffAcc && lastMatches.length}
          <button on:click={rePickAccount} class="btn btn-gold mt-3 text-xs"><UserRound size={13} /> {t('run.pickAccount')}</button>
        {/if}
      </div>
    {/if}

    {#if busyScan}
      <div class="fixed inset-0 z-50 flex flex-col items-center justify-center gap-3 bg-black/70 p-6">
        <Loader2 size={28} class="spin text-amber-200" />
        <p class="text-sm text-amber-200/90">{scanPhase || t('scan.loading')}</p>
      </div>
    {/if}

    {#if restoring}
      <div class="fixed inset-0 z-50 flex flex-col items-center justify-center gap-3 bg-black/70 p-6" role="status">
        <Loader2 size={28} class="spin text-amber-200" />
        <p class="text-sm text-amber-200/90">{restorePhase}</p>
      </div>
    {/if}

    {#if confirmOpen}
      <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4" role="dialog" aria-modal="true">
        <div class="panel w-full max-w-lg p-6">
          <p class="font-display text-lg font-semibold tracking-wide text-amber-200">{t('run.title')}</p>
          <pre class="mt-3 max-h-72 overflow-auto whitespace-pre-wrap text-xs leading-relaxed text-zinc-300">{confirmSummary}</pre>
          <div class="mt-5 flex justify-end gap-2">
            <button on:click={() => confirmOpen = false} class="btn btn-ghost">{t('run.cancel')}</button>
            <button on:click={doRun} class="btn btn-gold">{t('run.confirmBtn')}</button>
          </div>
        </div>
      </div>
    {/if}

    <footer class="mt-10 text-center text-xs text-zinc-500">
      <p class="flex items-center justify-center gap-1.5"><ShieldCheck size={14} /> {t('footer.private')}</p>
      <p class="mt-2 text-amber-200/60">{t('footer.tribute')} <a href="https://github.com/kvasszn/ree-save-editor" target="_blank" rel="noopener noreferrer" class="text-amber-200/80 underline hover:text-amber-100">kvasszn/ree-save-editor</a></p>
    </footer>
  </div>
</div>
