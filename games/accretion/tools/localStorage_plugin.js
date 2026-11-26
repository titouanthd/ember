// tools/localStorage_plugin.js
//
// Miniquad JS plugin exposing `localStorage` to Rust without wasm-bindgen.
// Must be loaded AFTER mq_js_bundle.js (it calls `miniquad_add_plugin`)
// and BEFORE `load("accretion.wasm")` (plugins register their imports then).
//
// Imports provided (see src/save.rs, all strings are null-terminated UTF-8):
//   ember_storage_get(key, out_buf, out_len) -> i32  bytes written, -1 on error
//   ember_storage_set(key, value)            -> i32  0 on success, -1 on error
//   ember_storage_remove(key)                -> i32  0 on success, -1 on error
//
// Memory goes through the bundle's `wasm_memory`. A fresh view is taken on
// every call because growing the wasm memory detaches old ArrayBuffers.

(function () {
    if (typeof miniquad_add_plugin !== 'function') {
        console.error('localStorage_plugin.js must be loaded AFTER mq_js_bundle.js');
        return;
    }

    const decoder = new TextDecoder('utf-8');
    const encoder = new TextEncoder();

    function heap() {
        return new Uint8Array(wasm_memory.buffer);
    }

    // Decodes the null-terminated UTF-8 string starting at `ptr`.
    function cString(ptr) {
        const mem = heap();
        let end = ptr;
        while (end < mem.length && mem[end] !== 0) {
            end++;
        }
        return decoder.decode(mem.subarray(ptr, end));
    }

    function ember_storage_get(key_ptr, out_buf_ptr, out_buf_len) {
        try {
            const value = localStorage.getItem(cString(key_ptr));
            if (value === null || out_buf_len === 0) {
                return 0;
            }
            const bytes = encoder.encode(value);
            if (bytes.length > out_buf_len - 1) {
                console.error('ember_storage_get: value (' + bytes.length +
                    ' bytes) exceeds the buffer (' + out_buf_len + ' bytes)');
                return -1;
            }
            const mem = heap();
            mem.set(bytes, out_buf_ptr);
            mem[out_buf_ptr + bytes.length] = 0;
            return bytes.length;
        } catch (e) {
            console.error('ember_storage_get failed:', e);
            return -1;
        }
    }

    function ember_storage_set(key_ptr, val_ptr) {
        try {
            localStorage.setItem(cString(key_ptr), cString(val_ptr));
            return 0;
        } catch (e) {
            console.error('ember_storage_set failed:', e);
            return -1;
        }
    }

    function ember_storage_remove(key_ptr) {
        try {
            localStorage.removeItem(cString(key_ptr));
            return 0;
        } catch (e) {
            console.error('ember_storage_remove failed:', e);
            return -1;
        }
    }

    miniquad_add_plugin({
        register_plugin: function (importObject) {
            importObject.env.ember_storage_get = ember_storage_get;
            importObject.env.ember_storage_set = ember_storage_set;
            importObject.env.ember_storage_remove = ember_storage_remove;
        },
        // Checked against the wasm export `ember_storage_crate_version`.
        version: 1,
        name: 'ember_storage',
    });
})();
