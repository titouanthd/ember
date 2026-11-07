// tools/localStorage_plugin.js
//
// Miniquad plugin exposing `localStorage` to Rust without wasm-bindgen.
// Must be loaded AFTER mq_js_bundle.js (it extends LibraryManager.library).
//
// See DESIGN.md §17 and BROWSER_BUILD.md §2.D.

(function () {
    if (typeof mergeInto !== 'function') {
        console.error('localStorage_plugin.js must be loaded AFTER mq_js_bundle.js');
        return;
    }

    mergeInto(LibraryManager.library, {
        ember_storage_get: function (key_ptr, out_buf_ptr, out_buf_len) {
            const key = UTF8ToString(key_ptr);
            try {
                const value = localStorage.getItem(key) || '';
                const bytes = new TextEncoder().encode(value);
                const n = Math.min(bytes.length, out_buf_len - 1);
                HEAPU8.set(bytes.subarray(0, n), out_buf_ptr);
                HEAPU8[out_buf_ptr + n] = 0; // null terminator
                return n;
            } catch (e) {
                console.error('ember_storage_get failed:', e);
                return -1;
            }
        },
        ember_storage_set: function (key_ptr, val_ptr) {
            const key = UTF8ToString(key_ptr);
            const value = UTF8ToString(val_ptr);
            try {
                localStorage.setItem(key, value);
                return 0;
            } catch (e) {
                console.error('ember_storage_set failed:', e);
                return -1;
            }
        },
        ember_storage_remove: function (key_ptr) {
            const key = UTF8ToString(key_ptr);
            try {
                localStorage.removeItem(key);
                return 0;
            } catch (e) {
                return -1;
            }
        },
    });
})();