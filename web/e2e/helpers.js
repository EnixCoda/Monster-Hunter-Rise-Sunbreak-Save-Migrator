// Shared mock of the File System Access API for e2e specs.
import { existsSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

export function haveFixtures() {
  const dir = path.join(path.dirname(fileURLToPath(import.meta.url)), 'fixtures');
  return existsSync(path.join(dir, 'switch_data001Slot.bin')) &&
         existsSync(path.join(dir, 'switch_data00-1.bin')) &&
         existsSync(path.join(dir, 'pc_data002Slot.bin')) &&
         existsSync(path.join(dir, 'pc_data00-1.bin'));
}
// Captures every write: window.__captures['<dirKey>/<fileName>'] = Uint8Array.
// Single-account mode (pcFiles) also keeps a legacy alias '<fileName>' for writes
// into win64_save. Multi-account mode keys captures as 'm<accountid>/<fileName>'.

export async function mockFsAccess(page, {
  switchFiles = {}, pcFiles = {}, preBackups = {}, cancelOn = 0, pcAsWin64 = false, multiAccounts = null, showIntro = false,
} = {}) {
  await page.addInitScript((cfg) => {
    const { switchFiles, pcFiles, preBackups, cancelOn, pcAsWin64, multiAccounts, showIntro } = cfg;
    if (!showIntro) { try { localStorage.setItem('mhsave_intro_done', '1'); } catch (e) {} }
    const multi = !!multiAccounts;
    window.__captures = {};
    const mkFile = (bytes) => ({ async arrayBuffer() { return new Uint8Array(bytes).buffer.slice(0); }, size: bytes.length, name: 'f' });

    const mkDir = (files, name = 'dir', capKey = null) => {
      const filesMap = { ...files };
      const keyPrefix = capKey || name;
      const h = {
        files: filesMap, name, __isdir: true,
        async getFileHandle(n, o) {
          if (!(n in filesMap) && !(o && o.create)) throw new DOMException('nf', 'NotFoundError');
          if (!(n in filesMap)) filesMap[n] = [];
          const data = filesMap[n];
          return {
            async getFile() { return mkFile(data); },
            async createWritable() {
              let b = new Uint8Array(0);
              return {
                async write(x) { b = new Uint8Array(x); },
                async close() {
                  filesMap[n] = Array.from(b);
                  window.__captures[keyPrefix + '/' + n] = b;
                  if (!multi && name === 'win64_save') window.__captures[n] = b;
                },
              };
            },
          };
        },
        async getDirectoryHandle(n, o) {
          if (o && o.create) { const d = mkDir({}, n); filesMap[n] = d; return d; }
          return Promise.reject(new DOMException('nf', 'NotFoundError'));
        },
        async removeEntry(n, o) { delete filesMap[n]; },
        entries: async function* () {
          for (const n of Object.keys(filesMap)) {
            const v = filesMap[n];
            if (v && v.__isdir) yield [n, v];
            else yield [n, { async getFile() { return mkFile(v); } }];
          }
        },
      };
      return h;
    };

    const makeWin64 = (files, backups, capKey) => {
      const w = mkDir(files || {}, 'win64_save', capKey);
      for (const [bn, bf] of Object.entries(backups || {})) w.files[bn] = mkDir(bf, bn);
      return w;
    };
    const makeNoSaveAccount = () => {
      const acc = mkDir({}, 'account');
      acc.getDirectoryHandle = async () => Promise.reject(new DOMException('nf', 'NotFoundError'));
      return acc;
    };
    const makeAccountChain = (win64) => {
      const r2 = mkDir({}, 'remote');
      r2.getDirectoryHandle = async (n) => (n === 'win64_save') ? win64 : Promise.reject(new DOMException('nf', 'NotFoundError'));
      const r144 = mkDir({}, '1446780');
      r144.getDirectoryHandle = async (n) => (n === 'remote') ? r2 : Promise.reject(new DOMException('nf', 'NotFoundError'));
      const acc = mkDir({}, 'account');
      acc.getDirectoryHandle = async (n) => (n === '1446780') ? r144 : Promise.reject(new DOMException('nf', 'NotFoundError'));
      return acc;
    };

    const accounts = multi ? multiAccounts : { '219758781': { files: pcFiles, backups: preBackups } };
    const userdata = mkDir({}, 'userdata');
    const accHandles = {};
    for (const [id, info] of Object.entries(accounts)) {
      accHandles[id] = info.noSave ? makeNoSaveAccount() : makeAccountChain(makeWin64(info.files, info.backups, multi ? 'm' + id : null));
    }
    userdata.entries = async function* () { for (const [id, acc] of Object.entries(accHandles)) yield [id, acc]; };

    const win64 = makeWin64(pcFiles, preBackups, null);
    const switchDir = mkDir(switchFiles, 'switchdump');

    let step = 0;
    window.showDirectoryPicker = async () => {
      step += 1;
      if (cancelOn > 0 && step === cancelOn) throw new DOMException('user cancelled', 'AbortError');
      return step === 1 ? switchDir : (pcAsWin64 && !multi ? win64 : userdata);
    };
  }, {
    switchFiles, pcFiles, preBackups, cancelOn, pcAsWin64, multiAccounts, showIntro,
  });
}

export function buffersEqual(a, b) {
  const A = a instanceof Uint8Array ? a : new Uint8Array(a);
  const B = b instanceof Uint8Array ? b : new Uint8Array(b);
  if (A.length !== B.length) return false;
  for (let i = 0; i < A.length; i++) if (A[i] !== B[i]) return false;
  return true;
}
