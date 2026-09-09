/* tslint:disable */
/* eslint-disable */

/**
 * Chroma subsampling format
 */
export enum ChromaSampling {
    /**
     * Both vertically and horizontally subsampled.
     */
    Cs420 = 0,
    /**
     * Horizontally subsampled.
     */
    Cs422 = 1,
    /**
     * Not subsampled.
     */
    Cs444 = 2,
    /**
     * Monochrome.
     */
    Cs400 = 3,
}

/**
 * Per-class content signature: returns a map of class-hash -> content-hash of the
 * decoded field values. Used to prove a source save and a migrated save carry the
 * same class content (content preservation) independent of ciphertext randomness.
 */
export function class_sigs(data: Uint8Array, steamid: bigint): any;

export function content_sig(data: Uint8Array, steamid: bigint): string;

/**
 * Copy one LOADINFO hunter entry (the title-screen character record: name, HR, MR,
 * play time and all nested per-character classes) from the source sys into the target
 * sys at the given index. The four link-derived enum fields are rewritten to the target
 * sys's own link so the entry stays consistent with the target's save header.
 */
export function copy_hunter_entry(tgt_sys: Uint8Array, src_sys: Uint8Array, src_steamid: bigint, src_curve: number, tgt_steamid: bigint, tgt_curve: number, dst_idx: number, src_idx: number): any;

/**
 * Decrypt + decompress a save and return the plaintext payload bytes (the
 * decrypted stream the game parses), terminates with any trailing NULs trimmed
 * so byte comparisons are stable. The crypto-layer per-write randomness is gone.
 */
export function decrypted_bytes(data: Uint8Array, steamid: bigint): any;

export function detect_curve(data: Uint8Array, steamid: bigint): number;

/**
 * Dump EditSaveData playerUIValue per slot: reports whether each slot has a
 */
export function edit_report(sys_data: Uint8Array, steamid: bigint): any;

/**
 * Dump every top-level class and its fields (hash + value kind), so we can locate
 * where appearance / playerUIValue / names live in the sys.
 */
export function field_dump(data: Uint8Array, steamid: bigint): any;

export function hunter_names(sys_data: Uint8Array, steamid: bigint): any;

export function migrate(switch_data: Uint8Array, slot_data: Uint8Array, sys_data: Uint8Array, steamid: bigint, curve: number): any;

/**
 * Steam -> Steam migrate preserving the target's account-bound identity: copies all
 * HASHES classes from the source EXCEPT the HunterRecord (0x355c8c4f, which carries the
 * account's HunterUniqueID / NetworkUniqueId / NsaID). Those are kept from the target
 * template so the resulting save belongs to the target account.
 */
export function migrate_acct(src: Uint8Array, src_steamid: bigint, template: Uint8Array, sys_data: Uint8Array, target_steamid: bigint, curve: number, tgt_curve: number): any;

/**
 * Copy the per-slot EditSaveData (character appearance / playerUIValue, slot name,
 * save time, slot-used flag) from the source sys into the target sys template, for the
 * given target slot indices. Writes the result keyed to target_steamid.
 */
export function migrate_sys(src_sys: Uint8Array, src_steamid: bigint, tgt_sys: Uint8Array, tgt_steamid: bigint, slots: Uint32Array, curve: number): any;

/**
 * Steam -> Steam migrate: read source save with src_steamid, merge classes into a target
 * template, write with target_steamid (identical code path to the validated switch->steam migrate).
 */
export function migrate_x(src: Uint8Array, src_steamid: bigint, template: Uint8Array, sys_data: Uint8Array, target_steamid: bigint, curve: number, tgt_curve: number): any;

export function patch_bytes(data: Uint8Array, steamid: bigint, curve: number, offset: number, bytes: Uint8Array): any;

export function patch_link(sys_data: Uint8Array, link: number): any;

export function raw_patch_str(sys_data: Uint8Array, steamid: bigint, curve: number, offset: number, name: string): any;

export function raw_patch_u32(sys_data: Uint8Array, steamid: bigint, curve: number, offset: number, value: number): any;

/**
 * Steam -> Steam re-key: decrypt with src_steamid, re-encrypt with target_steamid, set link.
 */
export function rekey(data: Uint8Array, src_steamid: bigint, target_steamid: bigint, link: number, curve: number): any;

export function set_both_names(sys_data: Uint8Array, steamid: bigint, idx: number, name: string): any;

export function set_hunter_name(sys_data: Uint8Array, steamid: bigint, idx: number, name: string): any;

export function set_slot_used(sys_data: Uint8Array, steamid: bigint, idx: number, used: boolean): any;

export function slot_names(sys_data: Uint8Array, steamid: bigint): any;

export function slot_rank(slot_data: Uint8Array, steamid: bigint): any;

export function slot_times(sys_data: Uint8Array, steamid: bigint): any;

/**
 * Read the EditSaveData `playerSlotUsed` boolean array (per-slot, length 5),
 * so we can see which save slots the title screen treats as occupied.
 */
export function slot_used_flags(sys_data: Uint8Array, steamid: bigint): any;

/**
 * Sync a target LOADINFO hunter entry (title-screen character record) with the same
 * entry from another sys (e.g. the Switch sys after a Switch -> Steam slot copy).
 * Every field present in both entries is taken from the source (name, HR/MR, play
 * time, buddies, outfit colours, and the whole eec7904b appearance class with
 * gender/face/hair/voice). Fields that must stay bound to the target are preserved:
 * the four link-derived enums (0x9eb56094/0xed59430c/0x8a471883/0xf4deaf65) and the
 * 16-byte character GUID (0xe40fc0cd). The result is re-serialised and re-keyed.
 */
export function sync_hunter_entry(sys_data: Uint8Array, src_sys: Uint8Array, steamid: bigint, curve: number, tgt_idx: number, src_idx: number): any;

export function validate_steam(slot: Uint8Array, sys: Uint8Array, steamid: bigint): string;

export function validate_switch(data: Uint8Array, steamid: bigint): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly class_sigs: (a: number, b: number, c: bigint) => [number, number, number];
    readonly content_sig: (a: number, b: number, c: bigint) => [number, number, number, number];
    readonly copy_hunter_entry: (a: number, b: number, c: number, d: number, e: bigint, f: number, g: bigint, h: number, i: number, j: number) => [number, number, number];
    readonly decrypted_bytes: (a: number, b: number, c: bigint) => [number, number, number];
    readonly detect_curve: (a: number, b: number, c: bigint) => [number, number, number];
    readonly edit_report: (a: number, b: number, c: bigint) => [number, number, number];
    readonly field_dump: (a: number, b: number, c: bigint) => [number, number, number];
    readonly hunter_names: (a: number, b: number, c: bigint) => any;
    readonly migrate: (a: number, b: number, c: number, d: number, e: number, f: number, g: bigint, h: number) => [number, number, number];
    readonly migrate_acct: (a: number, b: number, c: bigint, d: number, e: number, f: number, g: number, h: bigint, i: number, j: number) => [number, number, number];
    readonly migrate_sys: (a: number, b: number, c: bigint, d: number, e: number, f: bigint, g: number, h: number, i: number) => [number, number, number];
    readonly migrate_x: (a: number, b: number, c: bigint, d: number, e: number, f: number, g: number, h: bigint, i: number, j: number) => [number, number, number];
    readonly patch_bytes: (a: number, b: number, c: bigint, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly patch_link: (a: number, b: number, c: number) => [number, number, number];
    readonly raw_patch_str: (a: number, b: number, c: bigint, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly raw_patch_u32: (a: number, b: number, c: bigint, d: number, e: number, f: number) => [number, number, number];
    readonly rekey: (a: number, b: number, c: bigint, d: bigint, e: number, f: number) => [number, number, number];
    readonly set_both_names: (a: number, b: number, c: bigint, d: number, e: number, f: number) => [number, number, number];
    readonly set_hunter_name: (a: number, b: number, c: bigint, d: number, e: number, f: number) => [number, number, number];
    readonly set_slot_used: (a: number, b: number, c: bigint, d: number, e: number) => [number, number, number];
    readonly slot_names: (a: number, b: number, c: bigint) => any;
    readonly slot_rank: (a: number, b: number, c: bigint) => any;
    readonly slot_times: (a: number, b: number, c: bigint) => any;
    readonly slot_used_flags: (a: number, b: number, c: bigint) => [number, number, number];
    readonly sync_hunter_entry: (a: number, b: number, c: number, d: number, e: bigint, f: number, g: number, h: number) => [number, number, number];
    readonly validate_steam: (a: number, b: number, c: number, d: number, e: bigint) => [number, number, number, number];
    readonly validate_switch: (a: number, b: number, c: bigint) => [number, number, number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
