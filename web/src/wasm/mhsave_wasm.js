/* @ts-self-types="./mhsave_wasm.d.ts" */

/**
 * Chroma subsampling format
 * @enum {0 | 1 | 2 | 3}
 */
export const ChromaSampling = Object.freeze({
    /**
     * Both vertically and horizontally subsampled.
     */
    Cs420: 0, "0": "Cs420",
    /**
     * Horizontally subsampled.
     */
    Cs422: 1, "1": "Cs422",
    /**
     * Not subsampled.
     */
    Cs444: 2, "2": "Cs444",
    /**
     * Monochrome.
     */
    Cs400: 3, "3": "Cs400",
});

/**
 * Per-class content signature: returns a map of class-hash -> content-hash of the
 * decoded field values. Used to prove a source save and a migrated save carry the
 * same class content (content preservation) independent of ciphertext randomness.
 * @param {Uint8Array} data
 * @param {bigint} steamid
 * @returns {any}
 */
export function class_sigs(data, steamid) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.class_sigs(ptr0, len0, steamid);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} data
 * @param {bigint} steamid
 * @returns {string}
 */
export function content_sig(data, steamid) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.content_sig(ptr0, len0, steamid);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Copy one LOADINFO hunter entry (the title-screen character record: name, HR, MR,
 * play time and all nested per-character classes) from the source sys into the target
 * sys at the given index. The four link-derived enum fields are rewritten to the target
 * sys's own link so the entry stays consistent with the target's save header.
 * @param {Uint8Array} tgt_sys
 * @param {Uint8Array} src_sys
 * @param {bigint} src_steamid
 * @param {number} src_curve
 * @param {bigint} tgt_steamid
 * @param {number} tgt_curve
 * @param {number} dst_idx
 * @param {number} src_idx
 * @returns {any}
 */
export function copy_hunter_entry(tgt_sys, src_sys, src_steamid, src_curve, tgt_steamid, tgt_curve, dst_idx, src_idx) {
    const ptr0 = passArray8ToWasm0(tgt_sys, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(src_sys, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.copy_hunter_entry(ptr0, len0, ptr1, len1, src_steamid, src_curve, tgt_steamid, tgt_curve, dst_idx, src_idx);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Decrypt + decompress a save and return the plaintext payload bytes (the
 * decrypted stream the game parses), terminates with any trailing NULs trimmed
 * so byte comparisons are stable. The crypto-layer per-write randomness is gone.
 * @param {Uint8Array} data
 * @param {bigint} steamid
 * @returns {any}
 */
export function decrypted_bytes(data, steamid) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.decrypted_bytes(ptr0, len0, steamid);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} data
 * @param {bigint} steamid
 * @returns {number}
 */
export function detect_curve(data, steamid) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.detect_curve(ptr0, len0, steamid);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0] >>> 0;
}

/**
 * Dump EditSaveData playerUIValue per slot: reports whether each slot has a
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @returns {any}
 */
export function edit_report(sys_data, steamid) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.edit_report(ptr0, len0, steamid);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Dump every top-level class and its fields (hash + value kind), so we can locate
 * where appearance / playerUIValue / names live in the sys.
 * @param {Uint8Array} data
 * @param {bigint} steamid
 * @returns {any}
 */
export function field_dump(data, steamid) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.field_dump(ptr0, len0, steamid);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @returns {any}
 */
export function hunter_names(sys_data, steamid) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.hunter_names(ptr0, len0, steamid);
    return ret;
}

/**
 * @param {Uint8Array} switch_data
 * @param {Uint8Array} slot_data
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @param {number} curve
 * @returns {any}
 */
export function migrate(switch_data, slot_data, sys_data, steamid, curve) {
    const ptr0 = passArray8ToWasm0(switch_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(slot_data, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.migrate(ptr0, len0, ptr1, len1, ptr2, len2, steamid, curve);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Steam -> Steam migrate preserving the target's account-bound identity: copies all
 * HASHES classes from the source EXCEPT the HunterRecord (0x355c8c4f, which carries the
 * account's HunterUniqueID / NetworkUniqueId / NsaID). Those are kept from the target
 * template so the resulting save belongs to the target account.
 * @param {Uint8Array} src
 * @param {bigint} src_steamid
 * @param {Uint8Array} template
 * @param {Uint8Array} sys_data
 * @param {bigint} target_steamid
 * @param {number} curve
 * @param {number} tgt_curve
 * @returns {any}
 */
export function migrate_acct(src, src_steamid, template, sys_data, target_steamid, curve, tgt_curve) {
    const ptr0 = passArray8ToWasm0(src, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(template, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.migrate_acct(ptr0, len0, src_steamid, ptr1, len1, ptr2, len2, target_steamid, curve, tgt_curve);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Copy the per-slot EditSaveData (character appearance / playerUIValue, slot name,
 * save time, slot-used flag) from the source sys into the target sys template, for the
 * given target slot indices. Writes the result keyed to target_steamid.
 * @param {Uint8Array} src_sys
 * @param {bigint} src_steamid
 * @param {Uint8Array} tgt_sys
 * @param {bigint} tgt_steamid
 * @param {Uint32Array} slots
 * @param {number} curve
 * @returns {any}
 */
export function migrate_sys(src_sys, src_steamid, tgt_sys, tgt_steamid, slots, curve) {
    const ptr0 = passArray8ToWasm0(src_sys, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(tgt_sys, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passArray32ToWasm0(slots, wasm.__wbindgen_malloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.migrate_sys(ptr0, len0, src_steamid, ptr1, len1, tgt_steamid, ptr2, len2, curve);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Steam -> Steam migrate: read source save with src_steamid, merge classes into a target
 * template, write with target_steamid (identical code path to the validated switch->steam migrate).
 * @param {Uint8Array} src
 * @param {bigint} src_steamid
 * @param {Uint8Array} template
 * @param {Uint8Array} sys_data
 * @param {bigint} target_steamid
 * @param {number} curve
 * @param {number} tgt_curve
 * @returns {any}
 */
export function migrate_x(src, src_steamid, template, sys_data, target_steamid, curve, tgt_curve) {
    const ptr0 = passArray8ToWasm0(src, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(template, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.migrate_x(ptr0, len0, src_steamid, ptr1, len1, ptr2, len2, target_steamid, curve, tgt_curve);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} data
 * @param {bigint} steamid
 * @param {number} curve
 * @param {number} offset
 * @param {Uint8Array} bytes
 * @returns {any}
 */
export function patch_bytes(data, steamid, curve, offset, bytes) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.patch_bytes(ptr0, len0, steamid, curve, offset, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} sys_data
 * @param {number} link
 * @returns {any}
 */
export function patch_link(sys_data, link) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.patch_link(ptr0, len0, link);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @param {number} curve
 * @param {number} offset
 * @param {string} name
 * @returns {any}
 */
export function raw_patch_str(sys_data, steamid, curve, offset, name) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.raw_patch_str(ptr0, len0, steamid, curve, offset, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @param {number} curve
 * @param {number} offset
 * @param {number} value
 * @returns {any}
 */
export function raw_patch_u32(sys_data, steamid, curve, offset, value) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.raw_patch_u32(ptr0, len0, steamid, curve, offset, value);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Steam -> Steam re-key: decrypt with src_steamid, re-encrypt with target_steamid, set link.
 * @param {Uint8Array} data
 * @param {bigint} src_steamid
 * @param {bigint} target_steamid
 * @param {number} link
 * @param {number} curve
 * @returns {any}
 */
export function rekey(data, src_steamid, target_steamid, link, curve) {
    const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.rekey(ptr0, len0, src_steamid, target_steamid, link, curve);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @param {number} idx
 * @param {string} name
 * @returns {any}
 */
export function set_both_names(sys_data, steamid, idx, name) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.set_both_names(ptr0, len0, steamid, idx, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @param {number} idx
 * @param {string} name
 * @returns {any}
 */
export function set_hunter_name(sys_data, steamid, idx, name) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.set_hunter_name(ptr0, len0, steamid, idx, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @param {number} idx
 * @param {boolean} used
 * @returns {any}
 */
export function set_slot_used(sys_data, steamid, idx, used) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.set_slot_used(ptr0, len0, steamid, idx, used);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @returns {any}
 */
export function slot_names(sys_data, steamid) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.slot_names(ptr0, len0, steamid);
    return ret;
}

/**
 * @param {Uint8Array} slot_data
 * @param {bigint} steamid
 * @returns {any}
 */
export function slot_rank(slot_data, steamid) {
    const ptr0 = passArray8ToWasm0(slot_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.slot_rank(ptr0, len0, steamid);
    return ret;
}

/**
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @returns {any}
 */
export function slot_times(sys_data, steamid) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.slot_times(ptr0, len0, steamid);
    return ret;
}

/**
 * Read the EditSaveData `playerSlotUsed` boolean array (per-slot, length 5),
 * so we can see which save slots the title screen treats as occupied.
 * @param {Uint8Array} sys_data
 * @param {bigint} steamid
 * @returns {any}
 */
export function slot_used_flags(sys_data, steamid) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.slot_used_flags(ptr0, len0, steamid);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Sync a target LOADINFO hunter entry (title-screen character record) with the same
 * entry from another sys (e.g. the Switch sys after a Switch -> Steam slot copy).
 * Every field present in both entries is taken from the source (name, HR/MR, play
 * time, buddies, outfit colours, and the whole eec7904b appearance class with
 * gender/face/hair/voice). Fields that must stay bound to the target are preserved:
 * the four link-derived enums (0x9eb56094/0xed59430c/0x8a471883/0xf4deaf65) and the
 * 16-byte character GUID (0xe40fc0cd). The result is re-serialised and re-keyed.
 * @param {Uint8Array} sys_data
 * @param {Uint8Array} src_sys
 * @param {bigint} steamid
 * @param {number} curve
 * @param {number} tgt_idx
 * @param {number} src_idx
 * @returns {any}
 */
export function sync_hunter_entry(sys_data, src_sys, steamid, curve, tgt_idx, src_idx) {
    const ptr0 = passArray8ToWasm0(sys_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(src_sys, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.sync_hunter_entry(ptr0, len0, ptr1, len1, steamid, curve, tgt_idx, src_idx);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * @param {Uint8Array} slot
 * @param {Uint8Array} sys
 * @param {bigint} steamid
 * @returns {string}
 */
export function validate_steam(slot, sys, steamid) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passArray8ToWasm0(slot, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(sys, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.validate_steam(ptr0, len0, ptr1, len1, steamid);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * @param {Uint8Array} data
 * @param {bigint} steamid
 * @returns {string}
 */
export function validate_switch(data, steamid) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.validate_switch(ptr0, len0, steamid);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_is_undefined_6cff064c44e0d823: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_throw_bb96b2010945f0bc: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_getRandomValues_436a51d0629d84e1: function() { return handleError(function (arg0, arg1) {
            globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_new_116be93542d39019: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_ebe3e0f6837f0879: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_from_slice_3eea173078478cfe: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_now_e7c6795a7f81e10f: function(arg0) {
            const ret = arg0.now();
            return ret;
        },
        __wbg_performance_3fcf6e32a7e1ed0a: function(arg0) {
            const ret = arg0.performance;
            return ret;
        },
        __wbg_push_adb0107829f02d75: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_set_8155bb79a948541b: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_static_accessor_GLOBAL_THIS_466428f93b4eaa76: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_c7aea38d4de089bc: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_42d4fae05e59267a: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_e0db14a0eba6a812: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbindgen_cast_0000000000000001: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./mhsave_wasm_bg.js": import0,
    };
}

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getUint32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('mhsave_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
