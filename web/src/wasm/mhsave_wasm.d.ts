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

export function detect_curve(data: Uint8Array, steamid: bigint): number;

export function hunter_names(sys_data: Uint8Array, steamid: bigint): any;

export function migrate(switch_data: Uint8Array, slot_data: Uint8Array, sys_data: Uint8Array, steamid: bigint, curve: number): any;

export function slot_names(sys_data: Uint8Array, steamid: bigint): any;

export function slot_rank(slot_data: Uint8Array, steamid: bigint): any;

export function slot_times(sys_data: Uint8Array, steamid: bigint): any;

export function validate_steam(slot: Uint8Array, sys: Uint8Array, steamid: bigint): string;

export function validate_switch(data: Uint8Array, steamid: bigint): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly detect_curve: (a: number, b: number, c: bigint) => [number, number, number];
    readonly hunter_names: (a: number, b: number, c: bigint) => any;
    readonly migrate: (a: number, b: number, c: number, d: number, e: number, f: number, g: bigint, h: number) => [number, number, number];
    readonly slot_names: (a: number, b: number, c: bigint) => any;
    readonly slot_rank: (a: number, b: number, c: bigint) => any;
    readonly slot_times: (a: number, b: number, c: bigint) => any;
    readonly validate_steam: (a: number, b: number, c: number, d: number, e: bigint) => [number, number, number, number];
    readonly validate_switch: (a: number, b: number, c: bigint) => [number, number, number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
