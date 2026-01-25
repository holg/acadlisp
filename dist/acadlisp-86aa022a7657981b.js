let wasm;

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
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
    }
}

let WASM_VECTOR_LEN = 0;

const HP41CalculatorFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_hp41calculator_free(ptr >>> 0, 1));

const HP41TranspilerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_hp41transpiler_free(ptr >>> 0, 1));

const WasmEngineFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmengine_free(ptr >>> 0, 1));

/**
 * HP-41C style RPN calculator with 4-level stack
 */
export class HP41Calculator {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        HP41CalculatorFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_hp41calculator_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get_last_x() {
        const ret = wasm.hp41calculator_get_last_x(this.__wbg_ptr);
        return ret;
    }
    /**
     * Transpile ENTER key to LISP (duplicate X)
     * @returns {string}
     */
    lisp_enter() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_enter(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get T value as LISP
     * @returns {string}
     */
    lisp_get_t() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_get_t(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get X value as LISP
     * @returns {string}
     */
    lisp_get_x() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_get_x(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get Y value as LISP
     * @returns {string}
     */
    lisp_get_y() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_get_y(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get Z value as LISP
     * @returns {string}
     */
    lisp_get_z() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_get_z(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile power to LISP
     * @returns {string}
     */
    lisp_power() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_power(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    reciprocal() {
        wasm.hp41calculator_reciprocal(this.__wbg_ptr);
    }
    sigma_plus() {
        wasm.hp41calculator_sigma_plus(this.__wbg_ptr);
    }
    clear_sigma() {
        wasm.hp41calculator_clear_sigma(this.__wbg_ptr);
    }
    clear_stack() {
        wasm.hp41calculator_clear_stack(this.__wbg_ptr);
    }
    /**
     * @param {string} digit
     */
    enter_digit(digit) {
        const ptr0 = passStringToWasm0(digit, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.hp41calculator_enter_digit(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Transpile division to LISP
     * @returns {string}
     */
    lisp_divide() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_divide(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile LAST X to LISP
     * @returns {string}
     */
    lisp_last_x() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_last_x(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    sigma_minus() {
        wasm.hp41calculator_sigma_minus(this.__wbg_ptr);
    }
    /**
     * @returns {number}
     */
    test_x_eq_0() {
        const ret = wasm.hp41calculator_test_x_eq_0(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    test_x_eq_y() {
        const ret = wasm.hp41calculator_test_x_eq_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    test_x_gt_0() {
        const ret = wasm.hp41calculator_test_x_gt_0(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    test_x_gt_y() {
        const ret = wasm.hp41calculator_test_x_gt_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    test_x_le_y() {
        const ret = wasm.hp41calculator_test_x_le_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    test_x_lt_0() {
        const ret = wasm.hp41calculator_test_x_lt_0(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    test_x_lt_y() {
        const ret = wasm.hp41calculator_test_x_lt_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    test_x_ne_0() {
        const ret = wasm.hp41calculator_test_x_ne_0(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    test_x_ne_y() {
        const ret = wasm.hp41calculator_test_x_ne_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * Transpile clear X to LISP
     * @returns {string}
     */
    lisp_clear_x() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_clear_x(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    enter_decimal() {
        wasm.hp41calculator_enter_decimal(this.__wbg_ptr);
    }
    /**
     * @returns {boolean}
     */
    is_entry_mode() {
        const ret = wasm.hp41calculator_is_entry_mode(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Transpile multiplication to LISP
     * @returns {string}
     */
    lisp_multiply() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_multiply(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile subtraction to LISP
     * @returns {string}
     */
    lisp_subtract() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_subtract(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    polar_to_rect() {
        wasm.hp41calculator_polar_to_rect(this.__wbg_ptr);
    }
    recall_last_x() {
        wasm.hp41calculator_recall_last_x(this.__wbg_ptr);
    }
    rect_to_polar() {
        wasm.hp41calculator_rect_to_polar(this.__wbg_ptr);
    }
    enter_exponent() {
        wasm.hp41calculator_enter_exponent(this.__wbg_ptr);
    }
    /**
     * @returns {string}
     */
    format_display() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_format_display(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile roll down to LISP
     * @returns {string}
     */
    lisp_roll_down() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_roll_down(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    get_input_buffer() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_get_input_buffer(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    get_lcd_segments() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_get_lcd_segments(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile clear stack to LISP
     * @returns {string}
     */
    lisp_clear_stack() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_clear_stack(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    ln() {
        wasm.hp41calculator_ln(this.__wbg_ptr);
    }
    pi() {
        wasm.hp41calculator_pi(this.__wbg_ptr);
    }
    add() {
        wasm.hp41calculator_add(this.__wbg_ptr);
    }
    chs() {
        wasm.hp41calculator_chs(this.__wbg_ptr);
    }
    cos() {
        wasm.hp41calculator_cos(this.__wbg_ptr);
    }
    exp() {
        wasm.hp41calculator_exp(this.__wbg_ptr);
    }
    constructor() {
        const ret = wasm.hp41calculator_new();
        this.__wbg_ptr = ret >>> 0;
        HP41CalculatorFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    sin() {
        wasm.hp41calculator_sin(this.__wbg_ptr);
    }
    tan() {
        wasm.hp41calculator_tan(this.__wbg_ptr);
    }
    acos() {
        wasm.hp41calculator_acos(this.__wbg_ptr);
    }
    asin() {
        wasm.hp41calculator_asin(this.__wbg_ptr);
    }
    atan() {
        wasm.hp41calculator_atan(this.__wbg_ptr);
    }
    drop() {
        wasm.hp41calculator_drop(this.__wbg_ptr);
    }
    /**
     * @param {number} val
     */
    push(val) {
        wasm.hp41calculator_push(this.__wbg_ptr, val);
    }
    sqrt() {
        wasm.hp41calculator_sqrt(this.__wbg_ptr);
    }
    swap() {
        wasm.hp41calculator_swap(this.__wbg_ptr);
    }
    enter() {
        wasm.hp41calculator_enter(this.__wbg_ptr);
    }
    /**
     * @returns {number}
     */
    get_t() {
        const ret = wasm.hp41calculator_get_t(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get_x() {
        const ret = wasm.hp41calculator_get_x(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get_y() {
        const ret = wasm.hp41calculator_get_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get_z() {
        const ret = wasm.hp41calculator_get_z(this.__wbg_ptr);
        return ret;
    }
    log10() {
        wasm.hp41calculator_log10(this.__wbg_ptr);
    }
    pow10() {
        wasm.hp41calculator_pow10(this.__wbg_ptr);
    }
    power() {
        wasm.hp41calculator_power(this.__wbg_ptr);
    }
    divide() {
        wasm.hp41calculator_divide(this.__wbg_ptr);
    }
    square() {
        wasm.hp41calculator_square(this.__wbg_ptr);
    }
    clear_x() {
        wasm.hp41calculator_clear_x(this.__wbg_ptr);
    }
    /**
     * Transpile ln to LISP
     * @returns {string}
     */
    lisp_ln() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_ln(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile PI to LISP
     * @returns {string}
     */
    lisp_pi() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_pi(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    percent() {
        wasm.hp41calculator_percent(this.__wbg_ptr);
    }
    /**
     * @param {number} digits
     */
    set_eng(digits) {
        wasm.hp41calculator_set_eng(this.__wbg_ptr, digits);
    }
    /**
     * @param {number} digits
     */
    set_fix(digits) {
        wasm.hp41calculator_set_fix(this.__wbg_ptr, digits);
    }
    /**
     * @param {number} digits
     */
    set_sci(digits) {
        wasm.hp41calculator_set_sci(this.__wbg_ptr, digits);
    }
    /**
     * Transpile addition to LISP
     * @returns {string}
     */
    lisp_add() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_add(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile CHS to LISP
     * @returns {string}
     */
    lisp_chs() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_chs(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile cos to LISP
     * @returns {string}
     */
    lisp_cos() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_cos(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile 1/x to LISP
     * @returns {string}
     */
    lisp_inv() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_inv(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile log (base 10) to LISP
     * @returns {string}
     */
    lisp_log() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_log(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile sin to LISP
     * @returns {string}
     */
    lisp_sin() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_sin(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile tan to LISP
     * @returns {string}
     */
    lisp_tan() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_tan(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    multiply() {
        wasm.hp41calculator_multiply(this.__wbg_ptr);
    }
    subtract() {
        wasm.hp41calculator_subtract(this.__wbg_ptr);
    }
    backspace() {
        wasm.hp41calculator_backspace(this.__wbg_ptr);
    }
    /**
     * Transpile pushing a number to LISP
     * @param {number} val
     * @returns {string}
     */
    lisp_push(val) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_push(this.__wbg_ptr, val);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile sqrt to LISP
     * @returns {string}
     */
    lisp_sqrt() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_sqrt(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Transpile swap to LISP
     * @returns {string}
     */
    lisp_swap() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41calculator_lisp_swap(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    roll_down() {
        wasm.hp41calculator_roll_down(this.__wbg_ptr);
    }
}
if (Symbol.dispose) HP41Calculator.prototype[Symbol.dispose] = HP41Calculator.prototype.free;

export class HP41Transpiler {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        HP41TranspilerFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_hp41transpiler_free(ptr, 0);
    }
    /**
     * @param {string} label
     * @returns {boolean}
     */
    goto_label(label) {
        const ptr0 = passStringToWasm0(label, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.hp41transpiler_goto_label(this.__wbg_ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
     * @returns {boolean}
     */
    is_running() {
        const ret = wasm.hp41transpiler_is_running(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {boolean}
     */
    is_prgm_mode() {
        const ret = wasm.hp41transpiler_is_prgm_mode(this.__wbg_ptr);
        return ret !== 0;
    }
    op_roll_down() {
        wasm.hp41transpiler_op_roll_down(this.__wbg_ptr);
    }
    /**
     * @returns {string}
     */
    get_current_lisp() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41transpiler_get_current_lisp(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {boolean}
     */
    toggle_prgm_mode() {
        const ret = wasm.hp41transpiler_toggle_prgm_mode(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {number}
     */
    instruction_count() {
        const ret = wasm.hp41transpiler_instruction_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {number} line
     * @returns {string}
     */
    get_instruction_at(line) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41transpiler_get_instruction_at(this.__wbg_ptr, line);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    get_program_listing() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41transpiler_get_program_listing(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    get_current_instruction_display() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41transpiler_get_current_instruction_display(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    bst() {
        wasm.hp41transpiler_bst(this.__wbg_ptr);
    }
    constructor() {
        const ret = wasm.hp41transpiler_new();
        this.__wbg_ptr = ret >>> 0;
        HP41TranspilerFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    run() {
        wasm.hp41transpiler_run(this.__wbg_ptr);
    }
    /**
     * @returns {string}
     */
    sst() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41transpiler_sst(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {number} val
     */
    push(val) {
        wasm.hp41transpiler_push(this.__wbg_ptr, val);
    }
    stop() {
        wasm.hp41transpiler_stop(this.__wbg_ptr);
    }
    enter() {
        wasm.hp41transpiler_enter(this.__wbg_ptr);
    }
    /**
     * @returns {number}
     */
    get_t() {
        const ret = wasm.hp41transpiler_get_t(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get_x() {
        const ret = wasm.hp41transpiler_get_x(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get_y() {
        const ret = wasm.hp41transpiler_get_y(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get_z() {
        const ret = wasm.hp41transpiler_get_z(this.__wbg_ptr);
        return ret;
    }
    op_ln() {
        wasm.hp41transpiler_op_ln(this.__wbg_ptr);
    }
    op_pi() {
        wasm.hp41transpiler_op_pi(this.__wbg_ptr);
    }
    /**
     * @param {string} input
     * @returns {string}
     */
    parse(input) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(input, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.hp41transpiler_parse(this.__wbg_ptr, ptr0, len0);
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
    reset() {
        wasm.hp41transpiler_reset(this.__wbg_ptr);
    }
    /**
     * @param {number} val
     */
    set_x(val) {
        wasm.hp41transpiler_set_x(this.__wbg_ptr, val);
    }
    /**
     * @returns {number}
     */
    get_pc() {
        const ret = wasm.hp41transpiler_get_pc(this.__wbg_ptr);
        return ret >>> 0;
    }
    op_add() {
        wasm.hp41transpiler_op_add(this.__wbg_ptr);
    }
    op_chs() {
        wasm.hp41transpiler_op_chs(this.__wbg_ptr);
    }
    op_clx() {
        wasm.hp41transpiler_op_clx(this.__wbg_ptr);
    }
    op_cos() {
        wasm.hp41transpiler_op_cos(this.__wbg_ptr);
    }
    op_div() {
        wasm.hp41transpiler_op_div(this.__wbg_ptr);
    }
    op_exp() {
        wasm.hp41transpiler_op_exp(this.__wbg_ptr);
    }
    op_inv() {
        wasm.hp41transpiler_op_inv(this.__wbg_ptr);
    }
    op_log() {
        wasm.hp41transpiler_op_log(this.__wbg_ptr);
    }
    op_mul() {
        wasm.hp41transpiler_op_mul(this.__wbg_ptr);
    }
    op_pow() {
        wasm.hp41transpiler_op_pow(this.__wbg_ptr);
    }
    op_sin() {
        wasm.hp41transpiler_op_sin(this.__wbg_ptr);
    }
    op_sub() {
        wasm.hp41transpiler_op_sub(this.__wbg_ptr);
    }
    op_tan() {
        wasm.hp41transpiler_op_tan(this.__wbg_ptr);
    }
    /**
     * @param {number} pc
     */
    set_pc(pc) {
        wasm.hp41transpiler_goto_line(this.__wbg_ptr, pc);
    }
    op_acos() {
        wasm.hp41transpiler_op_acos(this.__wbg_ptr);
    }
    op_asin() {
        wasm.hp41transpiler_op_asin(this.__wbg_ptr);
    }
    op_atan() {
        wasm.hp41transpiler_op_atan(this.__wbg_ptr);
    }
    op_sqrt() {
        wasm.hp41transpiler_op_sqrt(this.__wbg_ptr);
    }
    op_swap() {
        wasm.hp41transpiler_op_swap(this.__wbg_ptr);
    }
    /**
     * @param {number} max_steps
     * @returns {string}
     */
    run_all(max_steps) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41transpiler_run_all(this.__wbg_ptr, max_steps);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    to_lisp() {
        let deferred2_0;
        let deferred2_1;
        try {
            const ret = wasm.hp41transpiler_to_lisp(this.__wbg_ptr);
            var ptr1 = ret[0];
            var len1 = ret[1];
            if (ret[3]) {
                ptr1 = 0; len1 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred2_0 = ptr1;
            deferred2_1 = len1;
            return getStringFromWasm0(ptr1, len1);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    get_name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.hp41transpiler_get_name(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    op_clear() {
        wasm.hp41transpiler_op_clear(this.__wbg_ptr);
    }
    op_exp10() {
        wasm.hp41transpiler_op_exp10(this.__wbg_ptr);
    }
    op_lastx() {
        wasm.hp41transpiler_op_lastx(this.__wbg_ptr);
    }
    /**
     * @returns {number}
     */
    get_lastx() {
        const ret = wasm.hp41transpiler_get_lastx(this.__wbg_ptr);
        return ret;
    }
    /**
     * @param {number} line
     */
    goto_line(line) {
        wasm.hp41transpiler_goto_line(this.__wbg_ptr, line);
    }
    op_square() {
        wasm.hp41transpiler_op_square(this.__wbg_ptr);
    }
}
if (Symbol.dispose) HP41Transpiler.prototype[Symbol.dispose] = HP41Transpiler.prototype.free;

/**
 * WASM-accessible AutoLISP engine
 */
export class WasmEngine {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmEngineFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmengine_free(ptr, 0);
    }
    /**
     * Get output buffer (PRINC/PRINT output)
     * @returns {string}
     */
    get_output() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_get_output(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get engine info
     * @returns {string}
     */
    engine_info() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_engine_info(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get example code by name
     * @param {string} name
     * @returns {string}
     */
    get_example(name) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_get_example(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get number of entities
     * @returns {number}
     */
    entity_count() {
        const ret = wasm.wasmengine_entity_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get current CAD type as string
     * @returns {string}
     */
    get_cad_type() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_get_cad_type(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Set CAD type: "rustlisp" (default) or "kicad"
     * This affects coordinate system handling in SVG output
     * @param {string} cad_type
     */
    set_cad_type(cad_type) {
        const ptr0 = passStringToWasm0(cad_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmengine_set_cad_type(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * Export current drawing as a KiCad Footprint
     * @param {string} footprint_name
     * @returns {string}
     */
    get_kicad_mod(footprint_name) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(footprint_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_get_kicad_mod(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Export current drawing as a KiCad Symbol Library
     * @param {string} library_name
     * @param {string} symbol_name
     * @returns {string}
     */
    get_kicad_sym(library_name, symbol_name) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(library_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(symbol_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_get_kicad_sym(this.__wbg_ptr, ptr0, len0, ptr1, len1);
            deferred3_0 = ret[0];
            deferred3_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Plot a function f(x) - evaluates the function and generates SVG
     * Returns JSON with { svg: string, points: number, min_y: number, max_y: number }
     * @param {string} code
     * @param {number} x_min
     * @param {number} x_max
     * @param {number} y_min
     * @param {number} y_max
     * @param {number} steps
     * @returns {string}
     */
    plot_function(code, x_min, x_max, y_min, y_max, steps) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(code, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_plot_function(this.__wbg_ptr, ptr0, len0, x_min, x_max, y_min, y_max, steps);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Generate DXF from current drawing entities
     * @returns {string}
     */
    get_entities_dxf() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_get_entities_dxf(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get drawing entities as SVG
     * @returns {string}
     */
    get_entities_svg() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_get_entities_svg(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get plot example code by name
     * @param {string} name
     * @returns {string}
     */
    get_plot_example(name) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_get_plot_example(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Get drawing entities as JSON array
     * @returns {string}
     */
    get_entities_json() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_get_entities_json(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get list of available examples
     * @returns {string}
     */
    get_example_names() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmengine_get_example_names(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Generate a Schaltplan drawing from spec
     * @param {string} name
     * @param {string} template
     * @param {string} components_json
     * @returns {string}
     */
    generate_schaltplan(name, template, components_json) {
        let deferred4_0;
        let deferred4_1;
        try {
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passStringToWasm0(template, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            const ptr2 = passStringToWasm0(components_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len2 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_generate_schaltplan(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2);
            deferred4_0 = ret[0];
            deferred4_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
        }
    }
    /**
     * Create a new engine instance
     */
    constructor() {
        const ret = wasm.wasmengine_new();
        this.__wbg_ptr = ret >>> 0;
        WasmEngineFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Execute AutoLISP code and return results as JSON
     * @param {string} code
     * @returns {string}
     */
    eval(code) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(code, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_eval(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Clear the drawing
     */
    clear() {
        wasm.wasmengine_clear(this.__wbg_ptr);
    }
    /**
     * Benchmark: run code multiple times and return timing stats as JSON
     * Returns: { "iterations": n, "total_ms": f, "avg_ms": f, "entities": n, "engine": "rust" }
     * @param {string} code
     * @param {number} iterations
     * @returns {string}
     */
    benchmark(code, iterations) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(code, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_benchmark(this.__wbg_ptr, ptr0, len0, iterations);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * Process CSV and return drawing specs as JSON
     * @param {string} csv_data
     * @returns {string}
     */
    parse_csv(csv_data) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(csv_data, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmengine_parse_csv(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
}
if (Symbol.dispose) WasmEngine.prototype[Symbol.dispose] = WasmEngine.prototype.free;

/**
 * Compress data using DEFLATE and return as base64
 * @param {string} data
 * @returns {string}
 */
export function compress_to_base64(data) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(data, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.compress_to_base64(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Decompress base64-encoded DEFLATE data
 * @param {string} data
 * @returns {string | undefined}
 */
export function decompress_from_base64(data) {
    const ptr0 = passStringToWasm0(data, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.decompress_from_base64(ptr0, len0);
    let v2;
    if (ret[0] !== 0) {
        v2 = getStringFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    }
    return v2;
}

/**
 * Extract encryption key from PNG image data using LSB steganography.
 * The key is embedded in the LSB of red channel pixels.
 * Format: 2-byte big-endian length + key bytes + null terminator
 * @param {Uint8Array} png_data
 * @returns {string | undefined}
 */
export function extract_key_from_png(png_data) {
    const ptr0 = passArray8ToWasm0(png_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.extract_key_from_png(ptr0, len0);
    let v2;
    if (ret[0] !== 0) {
        v2 = getStringFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    }
    return v2;
}

/**
 * Default encryption key (fallback if logo extraction fails)
 * @returns {string}
 */
export function get_default_key() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.get_default_key();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * XOR encrypt/decrypt text with a key
 * @param {string} text
 * @param {string} key
 * @returns {string}
 */
export function xor_crypt(text, key) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(key, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.xor_crypt(ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

const EXPECTED_RESPONSE_TYPES = new Set(['basic', 'cors', 'default']);

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && EXPECTED_RESPONSE_TYPES.has(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
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
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg___wbindgen_is_undefined_f6b95eab589e0269 = function(arg0) {
        const ret = arg0 === undefined;
        return ret;
    };
    imports.wbg.__wbg___wbindgen_throw_dd24417ed36fc46e = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbg_call_abb4ff46ce38be40 = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.call(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_instanceof_Window_b5cf7783caa68180 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_new_no_args_cb138f77cf6151ee = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbg_now_8cf15d6e317793e1 = function(arg0) {
        const ret = arg0.now();
        return ret;
    };
    imports.wbg.__wbg_performance_c77a440eff2efd9b = function(arg0) {
        const ret = arg0.performance;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_769e6b65d6557335 = function() {
        const ret = typeof global === 'undefined' ? null : global;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_THIS_60cf02db4de8e1c1 = function() {
        const ret = typeof globalThis === 'undefined' ? null : globalThis;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_SELF_08f5a74c69739274 = function() {
        const ret = typeof self === 'undefined' ? null : self;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_WINDOW_a8924b26aa92d024 = function() {
        const ret = typeof window === 'undefined' ? null : window;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbindgen_cast_2241b6af4c4b2941 = function(arg0, arg1) {
        // Cast intrinsic for `Ref(String) -> Externref`.
        const ret = getStringFromWasm0(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbindgen_init_externref_table = function() {
        const table = wasm.__wbindgen_externrefs;
        const offset = table.grow(4);
        table.set(0, undefined);
        table.set(offset + 0, undefined);
        table.set(offset + 1, null);
        table.set(offset + 2, true);
        table.set(offset + 3, false);
    };

    return imports;
}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedUint8ArrayMemory0 = null;


    wasm.__wbindgen_start();
    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
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


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('acadlisp_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync };
export default __wbg_init;
