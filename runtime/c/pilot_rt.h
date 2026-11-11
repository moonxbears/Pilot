/* ============================================================================
 * FILE: runtime/c/pilot_rt.h
 * The runtime of Pilot's native C11 backend (`pilot emit c`, `pilot build`).
 * ============================================================================
 * Every C file `pilot emit c` writes includes this header, and `pilot build`
 * passes its directory to the C compiler. It is C11 (and, up to the dynamic
 * values, C++17 for `pilot emit cpp`), and needs only the C library.
 *
 * Scalars. Integers, floats and booleans stay C scalars. Arithmetic helpers
 * stop the program on a zero divisor and wrap as the seed does.
 *
 * Dynamic values (`pilot_val`). A value whose physics is not a scalar (a string, a
 * list, a map, a record, a function value) or is unknown is one 16-byte
 * `pilot_val`: a tag, a string's length, and a payload. A struct value is a
 * C struct the emitted module declares (see "Records"); other records and
 * function values are lists of their fields, a function value's field 0
 * being its index in the module's function table. This is the registered
 * stand-in `emit.c_dynamic_values`: the design lays every value out by its
 * physics and holds value types in place. A string's length is 64 bits (see
 * "Strings").
 *
 * Memory is allocated and never freed; reference counting is F7.
 *
 * ABI. The entry points named `pilot_*` and `pilot_v_*` below are the stable
 * surface emitted code calls. Behaviour (printing, equality, ordering,
 * errors) follows the seed VM, whose conformance corpus checks it.
 * ========================================================================= */
#ifndef PILOT_RT_H
#define PILOT_RT_H

#include <inttypes.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef PILOT_NEGATIVE_EXPONENT
#define PILOT_NEGATIVE_EXPONENT "a negative integer exponent gives a float"
#endif

/* ---- Errors ------------------------------------------------------------- */

static inline void pilot_trap(const char *message) {
    fflush(stdout);
    fprintf(stderr, "runtime error: %s\n", message);
    exit(1);
}

/* ---- Integer arithmetic ------------------------------------------------- */

static inline uint64_t pilot_ipow(int64_t a, int64_t b) {
    if (b < 0) pilot_trap(PILOT_NEGATIVE_EXPONENT);
    uint64_t x = (uint64_t)a, r = 1, n = (uint64_t)b > 4294967295u ? 4294967295u : (uint64_t)b;
    while (n) {
        if (n & 1) r *= x;
        x *= x;
        n >>= 1;
    }
    return r;
}
static inline int64_t pilot_sdiv(int64_t a, int64_t b) {
    if (b == 0) pilot_trap("integer division by zero");
    if (b == -1) return (int64_t)(0u - (uint64_t)a);
    return a / b;
}
static inline int64_t pilot_srem(int64_t a, int64_t b) {
    if (b == 0) pilot_trap("integer division by zero");
    if (b == -1) return 0;
    return a % b;
}
static inline uint64_t pilot_udiv(uint64_t a, uint64_t b) {
    if (b == 0) pilot_trap("integer division by zero");
    return a / b;
}
static inline uint64_t pilot_urem(uint64_t a, uint64_t b) {
    if (b == 0) pilot_trap("integer division by zero");
    return a % b;
}

/* ---- Printing scalars --------------------------------------------------- */

/* The shortest digits that read back as x, written without an exponent, as
   the seed shows floats; the text goes to `out` (at least 400 bytes). */
static inline int pilot_fmt_f64(char *out, double x) {
    if (x != x) return sprintf(out, "NaN");
    if (isinf(x)) return sprintf(out, "%s", x > 0 ? "inf" : "-inf");
    if (x == 0) return sprintf(out, "%s", signbit(x) ? "-0" : "0");
    char buf[64];
    for (int p = 1; p <= 17; p++) {
        snprintf(buf, sizeof buf, "%.*e", p - 1, x);
        if (strtod(buf, NULL) == x) break;
    }
    char digits[32];
    int nd = 0, at = 0;
    const char *s = buf;
    if (*s == '-') {
        out[at++] = '-';
        s++;
    }
    while (*s && *s != 'e') {
        if (*s != '.') digits[nd++] = *s;
        s++;
    }
    int point = atoi(s + 1) + 1;
    while (nd > 1 && digits[nd - 1] == '0') nd--;
    if (point <= 0) {
        out[at++] = '0';
        out[at++] = '.';
        for (int i = 0; i < -point; i++) out[at++] = '0';
        memcpy(out + at, digits, (size_t)nd);
        at += nd;
    } else if (point >= nd) {
        memcpy(out + at, digits, (size_t)nd);
        at += nd;
        for (int i = nd; i < point; i++) out[at++] = '0';
    } else {
        memcpy(out + at, digits, (size_t)point);
        at += point;
        out[at++] = '.';
        memcpy(out + at, digits + point, (size_t)(nd - point));
        at += nd - point;
    }
    out[at] = 0;
    return at;
}
static inline void pilot_put_i64(FILE *f, int64_t x) { fprintf(f, "%" PRId64, x); }
static inline void pilot_put_u64(FILE *f, uint64_t x) { fprintf(f, "%" PRIu64, x); }
static inline void pilot_put_bool(FILE *f, bool x) { fputs(x ? "true" : "false", f); }
static inline void pilot_put_str(FILE *f, const char *x) { fputs(x, f); }
static inline void pilot_put_f64(FILE *f, double x) {
    char buf[400];
    int n = pilot_fmt_f64(buf, x);
    fwrite(buf, 1, (size_t)n, f);
}

#ifndef __cplusplus

/* ---- Dynamic values ----------------------------------------------------- */

enum {
    PILOT_V_NULL_TAG = 0,
    PILOT_V_BOOL,
    PILOT_V_INT,
    PILOT_V_UINT,
    PILOT_V_FLOAT,
    PILOT_V_STR,
    PILOT_V_LIST,
    PILOT_V_MAP,
    /* A struct value laid out as a C struct (see "Records" below). */
    PILOT_V_REC,
};

struct pilot_v_list;
struct pilot_v_map;
struct pilot_v_rec;

typedef struct pilot_val {
    uint32_t tag;
    /* A string's byte length; PILOT_V_LONG when it is longer than
       PILOT_V_SHORT_MAX and its length is the 64-bit word before its bytes. */
    uint32_t n;
    union {
        int64_t i;
        uint64_t u;
        double f;
        bool b;
        const char *s;
        struct pilot_v_list *l;
        struct pilot_v_map *m;
        struct pilot_v_rec *r;
    } as;
} pilot_val;

typedef struct pilot_v_list {
    int64_t len;
    int64_t cap;
    pilot_val *items;
} pilot_v_list;

typedef struct pilot_v_map_entry {
    pilot_val key;
    pilot_val val;
    uint64_t hash;
    /* 0 empty, 1 used, 2 removed. */
    uint8_t state;
} pilot_v_map_entry;

typedef struct pilot_v_map {
    int64_t count;
    int64_t used;
    int64_t cap;
    pilot_v_map_entry *slots;
} pilot_v_map;

/* A function value's code: the arguments, then the captured values. */
typedef pilot_val (*pilot_v_fn)(int32_t argc, pilot_val *argv);

#define PILOT_V_NULL ((pilot_val){0})
/*
 * Strings. A string is its bytes (followed by a 0 byte that is not part of
 * it) and a 64-bit length. A length up to PILOT_V_SHORT_MAX is kept in the
 * value itself; a longer one in the 8 bytes before the string's bytes, with
 * `n` set to PILOT_V_LONG. Tests lower PILOT_V_SHORT_MAX to run the long
 * form on small strings.
 */
#define PILOT_V_LONG UINT32_MAX
#ifndef PILOT_V_SHORT_MAX
#define PILOT_V_SHORT_MAX (UINT32_MAX - 1)
#endif

static inline size_t pilot_v_len(pilot_val v) {
    return v.n != PILOT_V_LONG ? (size_t)v.n : (size_t)((const uint64_t *)(const void *)v.as.s)[-1];
}

static inline void *pilot_alloc(size_t size);

/* A string of `len` bytes, uninitialised, for the caller to fill. */
static inline pilot_val pilot_v_strbuf(size_t len, char **bytes) {
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_STR;
    char *s;
    if (len <= PILOT_V_SHORT_MAX) {
        s = (char *)pilot_alloc(len + 1);
        v.n = (uint32_t)len;
    } else {
        uint64_t *base = (uint64_t *)pilot_alloc(sizeof(uint64_t) + len + 1);
        base[0] = (uint64_t)len;
        s = (char *)(base + 1);
        v.n = PILOT_V_LONG;
    }
    s[len] = 0;
    v.as.s = s;
    *bytes = s;
    return v;
}

/* Bytes `bytes[0..len)` as a string that shares them when its length fits
   in the value; else a copy in the long form. */
static inline pilot_val pilot_v_borrow(const char *bytes, size_t len) {
    if (len <= PILOT_V_SHORT_MAX) {
        pilot_val v = PILOT_V_NULL;
        v.tag = PILOT_V_STR;
        v.n = (uint32_t)len;
        v.as.s = bytes;
        return v;
    }
    char *s;
    pilot_val v = pilot_v_strbuf(len, &s);
    memcpy(s, bytes, len);
    return v;
}
/* A string literal, without copying it. */
#define PILOT_V_LIT(lit) pilot_v_borrow((lit), sizeof(lit) - 1)

static inline void *pilot_alloc(size_t size) {
    void *p = malloc(size ? size : 1);
    if (!p) pilot_trap("out of memory");
    return p;
}

/*
 * Small fixed-size objects (struct values): carved in 16-byte steps from
 * 64 KiB chunks rather than one malloc each. They are never handed back
 * one by one; F7's freeing gives this allocator per-size free lists.
 */
#define PILOT_OBJ_CHUNK ((size_t)64 * 1024)
static char *pilot_obj_at, *pilot_obj_end;
static inline void *pilot_alloc_obj(size_t size) {
    size = (size + 15) & ~(size_t)15;
    if (size > PILOT_OBJ_CHUNK / 4) return pilot_alloc(size);
    if ((size_t)(pilot_obj_end - pilot_obj_at) < size) {
        pilot_obj_at = (char *)pilot_alloc(PILOT_OBJ_CHUNK);
        pilot_obj_end = pilot_obj_at + PILOT_OBJ_CHUNK;
    }
    void *p = pilot_obj_at;
    pilot_obj_at += size;
    return p;
}

static inline pilot_val pilot_v_bool(bool b) {
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_BOOL;
    v.as.b = b;
    return v;
}
static inline pilot_val pilot_v_int(int64_t i) {
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_INT;
    v.as.i = i;
    return v;
}
static inline pilot_val pilot_v_uint(uint64_t u) {
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_UINT;
    v.as.u = u;
    return v;
}
static inline pilot_val pilot_v_float(double f) {
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_FLOAT;
    v.as.f = f;
    return v;
}
/* A string of `len` bytes copied from `bytes`. */
static inline pilot_val pilot_v_str(const char *bytes, size_t len) {
    char *s;
    pilot_val v = pilot_v_strbuf(len, &s);
    if (len) memcpy(s, bytes, len);
    return v;
}
static inline pilot_val pilot_v_cstr(const char *s) { return pilot_v_str(s, strlen(s)); }

static inline bool pilot_v_is_num(pilot_val v) { return v.tag == PILOT_V_INT || v.tag == PILOT_V_UINT || v.tag == PILOT_V_FLOAT; }
static inline bool pilot_v_is_intlike(pilot_val v) { return v.tag == PILOT_V_INT || v.tag == PILOT_V_UINT; }

/* The seed's name for the kind of `v`, in run-time errors. */
static inline const char *pilot_v_kind(pilot_val v) {
    switch (v.tag) {
    case PILOT_V_NULL_TAG: return "null";
    case PILOT_V_BOOL: return "boolean";
    case PILOT_V_INT:
    case PILOT_V_UINT: return "integer";
    case PILOT_V_FLOAT: return "float";
    case PILOT_V_STR: return "string";
    case PILOT_V_LIST: return "list";
    default: return "record";  /* a map or a struct value */
    }
}

/* ---- Records ------------------------------------------------------------ */

/*
 * A value of a struct type the program builds is a C struct the emitted
 * module declares from the layout's fields (`pilot_r<layout>`): this header,
 * then each field in its own C type. The module also gives, per layout, the
 * field count and functions reading and writing a field as a dynamic value,
 * in `pilot_rt_layouts`, indexed by layout id; the generic operations
 * (printing, comparing, a field read the emitted code could not type) go
 * through those.
 */
typedef struct pilot_v_rec {
    uint32_t layout;
} pilot_v_rec;

typedef struct pilot_v_layout {
    int32_t n;
    pilot_val (*get)(pilot_val rec, int32_t i);
    void (*set)(pilot_val rec, int32_t i, pilot_val v);
} pilot_v_layout;

/* Defined by the emitted module when it has record layouts. */
static const pilot_v_layout *pilot_rt_layouts;

static inline pilot_val pilot_v_rec_val(pilot_v_rec *h) {
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_REC;
    v.as.r = h;
    return v;
}
static inline const pilot_v_layout *pilot_v_rec_layout(pilot_val v) { return &pilot_rt_layouts[v.as.r->layout]; }
static inline pilot_val pilot_v_rec_get(pilot_val v, int32_t i) {
    const pilot_v_layout *l = pilot_v_rec_layout(v);
    if (i < 0 || i >= l->n) pilot_trap("field index out of range");
    return l->get(v, i);
}

/* ---- Text --------------------------------------------------------------- */

/* A growing byte buffer. */
typedef struct pilot_v_buf {
    char *data;
    size_t len;
    size_t cap;
} pilot_v_buf;

static inline void pilot_v_buf_add(pilot_v_buf *b, const char *s, size_t n) {
    if (b->len + n + 1 > b->cap) {
        size_t cap = b->cap ? b->cap : 32;
        while (b->len + n + 1 > cap) cap *= 2;
        char *d = (char *)pilot_alloc(cap);
        if (b->len) memcpy(d, b->data, b->len);
        free(b->data);
        b->data = d;
        b->cap = cap;
    }
    if (n) memcpy(b->data + b->len, s, n);
    b->len += n;
    b->data[b->len] = 0;
}
static inline void pilot_v_buf_cstr(pilot_v_buf *b, const char *s) { pilot_v_buf_add(b, s, strlen(s)); }

static void pilot_v_show_into(pilot_v_buf *b, pilot_val v);

static inline int pilot_v_cmp_text(const void *a, const void *b) {
    const pilot_val *x = (const pilot_val *)a, *y = (const pilot_val *)b;
    size_t xn = pilot_v_len(*x), yn = pilot_v_len(*y);
    int c = memcmp(x->as.s, y->as.s, xn < yn ? xn : yn);
    if (c) return c;
    return xn < yn ? -1 : xn > yn ? 1 : 0;
}

static inline pilot_val pilot_v_show(pilot_val v) {
    if (v.tag == PILOT_V_STR) return v;
    pilot_v_buf b = {0};
    pilot_v_show_into(&b, v);
    pilot_val out = pilot_v_str(b.data ? b.data : "", b.len);
    free(b.data);
    return out;
}

static void pilot_v_show_into(pilot_v_buf *b, pilot_val v) {
    char tmp[400];
    switch (v.tag) {
    case PILOT_V_NULL_TAG: pilot_v_buf_cstr(b, "null"); return;
    case PILOT_V_BOOL: pilot_v_buf_cstr(b, v.as.b ? "true" : "false"); return;
    case PILOT_V_INT: snprintf(tmp, sizeof tmp, "%" PRId64, v.as.i); pilot_v_buf_cstr(b, tmp); return;
    case PILOT_V_UINT: snprintf(tmp, sizeof tmp, "%" PRIu64, v.as.u); pilot_v_buf_cstr(b, tmp); return;
    case PILOT_V_FLOAT: pilot_v_buf_add(b, tmp, (size_t)pilot_fmt_f64(tmp, v.as.f)); return;
    case PILOT_V_STR: pilot_v_buf_add(b, v.as.s, pilot_v_len(v)); return;
    case PILOT_V_LIST:
        pilot_v_buf_cstr(b, "[");
        for (int64_t i = 0; i < v.as.l->len; i++) {
            if (i) pilot_v_buf_cstr(b, ", ");
            pilot_v_show_into(b, v.as.l->items[i]);
        }
        pilot_v_buf_cstr(b, "]");
        return;
    case PILOT_V_REC: {
        /* As a record of dynamic fields is shown: its fields in brackets. */
        int32_t n = pilot_v_rec_layout(v)->n;
        pilot_v_buf_cstr(b, "[");
        for (int32_t i = 0; i < n; i++) {
            if (i) pilot_v_buf_cstr(b, ", ");
            pilot_v_show_into(b, pilot_v_rec_get(v, i));
        }
        pilot_v_buf_cstr(b, "]");
        return;
    }
    default: {
        /* `{ k: v, .. }` with the entries' texts in order. */
        pilot_v_map *m = v.as.m;
        pilot_val *parts = (pilot_val *)pilot_alloc(sizeof(pilot_val) * (size_t)(m->count ? m->count : 1));
        int64_t n = 0;
        for (int64_t i = 0; i < m->cap; i++) {
            if (m->slots[i].state != 1) continue;
            pilot_v_buf e = {0};
            pilot_v_buf_add(&e, m->slots[i].key.as.s, pilot_v_len(m->slots[i].key));
            pilot_v_buf_cstr(&e, ": ");
            pilot_v_show_into(&e, m->slots[i].val);
            parts[n++] = pilot_v_str(e.data ? e.data : "", e.len);
            free(e.data);
        }
        qsort(parts, (size_t)n, sizeof(pilot_val), pilot_v_cmp_text);
        pilot_v_buf_cstr(b, "{ ");
        for (int64_t i = 0; i < n; i++) {
            if (i) pilot_v_buf_cstr(b, ", ");
            pilot_v_buf_add(b, parts[i].as.s, pilot_v_len(parts[i]));
        }
        pilot_v_buf_cstr(b, " }");
        free(parts);
        return;
    }
    }
}

static inline void pilot_v_put(FILE *f, pilot_val v) {
    if (v.tag == PILOT_V_STR) {
        fwrite(v.as.s, 1, pilot_v_len(v), f);
        return;
    }
    pilot_val s = pilot_v_show(v);
    fwrite(s.as.s, 1, pilot_v_len(s), f);
}

/*
 * Appending in place. Strings are immutable, so `a + b` may write `b` just
 * past `a` when `a` ends where its buffer's written bytes end: every other
 * string in that buffer is a prefix of `a`, and its bytes do not change.
 * The runtime remembers the buffers concatenation made most recently (the
 * largest are kept), with how far each is written and its capacity, and a
 * buffer that runs out of room is replaced by one twice as large, so a loop
 * of `s = s + x` copies O(n) bytes in all rather than O(n^2). Only strings
 * whose length is kept in the value take part (a long string's length
 * lives in its buffer, which the strings sharing it read). F7, which frees
 * buffers, must forget a buffer here when it frees it.
 */
#ifndef PILOT_V_GROW_SLOTS
#define PILOT_V_GROW_SLOTS 8
#endif
typedef struct pilot_v_grow {
    const char *base;
    size_t fill;
    size_t cap;
} pilot_v_grow;
static pilot_v_grow pilot_v_grows[PILOT_V_GROW_SLOTS];

static inline pilot_val pilot_v_concat(pilot_val a, pilot_val b) {
    if (a.tag != PILOT_V_STR) a = pilot_v_show(a);
    if (b.tag != PILOT_V_STR) b = pilot_v_show(b);
    size_t an = pilot_v_len(a), bn = pilot_v_len(b), n = an + bn;
    char *s;
    if (a.n == PILOT_V_LONG || n > PILOT_V_SHORT_MAX) {
        pilot_val v = pilot_v_strbuf(n, &s);
        memcpy(s, a.as.s, an);
        memcpy(s + an, b.as.s, bn);
        return v;
    }
    /* The buffer `a` ends at the written end of, if any; else the slot of
       the smallest buffer, which the new one replaces. */
    int slot = 0;
    bool grows = false;
    for (int i = 0; i < PILOT_V_GROW_SLOTS; i++) {
        pilot_v_grow *g = &pilot_v_grows[i];
        if (g->base == a.as.s && g->fill == an) {
            if (n <= g->cap) {
                char *at = (char *)g->base;
                memcpy(at + an, b.as.s, bn);
                at[n] = 0;
                g->fill = n;
                pilot_val v = a;
                v.n = (uint32_t)n;
                return v;
            }
            slot = i;
            grows = true;
            break;
        }
        if (g->cap < pilot_v_grows[slot].cap) slot = i;
    }
    /* A string that keeps growing gets twice the room it needs. */
    size_t cap = grows ? n * 2 : n;
    if (cap > PILOT_V_SHORT_MAX) cap = PILOT_V_SHORT_MAX;
    s = (char *)pilot_alloc(cap + 1);
    memcpy(s, a.as.s, an);
    memcpy(s + an, b.as.s, bn);
    s[n] = 0;
    pilot_v_grows[slot] = (pilot_v_grow){s, n, cap};
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_STR;
    v.n = (uint32_t)n;
    v.as.s = s;
    return v;
}

/* ---- Conversions -------------------------------------------------------- */

/* A value the program's types say is a number: its integer bits. */
static inline int64_t pilot_v_as_i64(pilot_val v) {
    if (pilot_v_is_intlike(v)) return v.as.i;
    if (v.tag == PILOT_V_BOOL) return v.as.b;
    if (v.tag == PILOT_V_FLOAT) return (int64_t)v.as.f;
    char m[96];
    snprintf(m, sizeof m, "expected an integer, found %s", pilot_v_kind(v));
    pilot_trap(m);
    return 0;
}
static inline double pilot_v_as_f64(pilot_val v) {
    if (v.tag == PILOT_V_FLOAT) return v.as.f;
    if (v.tag == PILOT_V_INT) return (double)v.as.i;
    if (v.tag == PILOT_V_UINT) return (double)v.as.u;
    char m[96];
    snprintf(m, sizeof m, "expected a float, found %s", pilot_v_kind(v));
    pilot_trap(m);
    return 0;
}
static inline bool pilot_v_truth(pilot_val v) {
    if (v.tag != PILOT_V_BOOL) {
        char m[96];
        snprintf(m, sizeof m, "condition must be a boolean, found %s", pilot_v_kind(v));
        pilot_trap(m);
    }
    return v.as.b;
}

/* `v as <number>`: checked as the seed checks it. */
static inline void pilot_v_not_number(pilot_val v) {
    char m[96];
    snprintf(m, sizeof m, "cannot convert %s to a number", pilot_v_kind(v));
    pilot_trap(m);
}
static inline bool pilot_v_to_bool(pilot_val v) {
    if (v.tag == PILOT_V_BOOL) return v.as.b;
    if (v.tag == PILOT_V_FLOAT) return v.as.f != 0;
    if (pilot_v_is_intlike(v)) return v.as.i != 0;
    pilot_v_not_number(v);
    return false;
}
static inline double pilot_v_to_f64(pilot_val v) {
    if (v.tag == PILOT_V_BOOL) return v.as.b ? 1 : 0;
    if (!pilot_v_is_num(v)) pilot_v_not_number(v);
    return pilot_v_as_f64(v);
}
/* An integer of `bits` bits, signed or not, as 64 bits the caller narrows. */
static inline int64_t pilot_v_to_int(pilot_val v, int bits, bool is_signed) {
    if (v.tag == PILOT_V_BOOL) return v.as.b;
    if (pilot_v_is_intlike(v)) return v.as.i;
    if (v.tag != PILOT_V_FLOAT) pilot_v_not_number(v);
    double t = trunc(v.as.f);
    double hi = is_signed ? 9223372036854775808.0 : 18446744073709551616.0;
    if (!(t >= -9223372036854775808.0 && t < hi)) {
        char m[480];
        char num[400];
        pilot_fmt_f64(num, v.as.f);
        snprintf(m, sizeof m, "%s does not fit an integer", num);
        pilot_trap(m);
    }
    (void)bits;
    if (!is_signed && t >= 9223372036854775808.0) return (int64_t)(uint64_t)t;
    return (int64_t)t;
}

/* ---- Arithmetic on dynamic values --------------------------------------- */

static inline void pilot_v_type_error(const char *sym, pilot_val a, pilot_val b) {
    char m[160];
    snprintf(m, sizeof m, "cannot apply `%s` to %s and %s", sym, pilot_v_kind(a), pilot_v_kind(b));
    pilot_trap(m);
}

/* `+ - * / %` (`op`) as the seed computes them on values of any kind. */
static inline pilot_val pilot_v_arith(char op, pilot_val a, pilot_val b) {
    char sym[2] = {op, 0};
    if (!(pilot_v_is_num(a) && pilot_v_is_num(b))) pilot_v_type_error(sym, a, b);
    if (pilot_v_is_intlike(a) && pilot_v_is_intlike(b)) {
        uint64_t x = (uint64_t)a.as.i, y = (uint64_t)b.as.i;
        switch (op) {
        case '+': return pilot_v_int((int64_t)(x + y));
        case '-': return pilot_v_int((int64_t)(x - y));
        case '*': return pilot_v_int((int64_t)(x * y));
        case '/': return pilot_v_int(pilot_sdiv(a.as.i, b.as.i));
        default: return pilot_v_int(pilot_srem(a.as.i, b.as.i));
        }
    }
    double x = pilot_v_as_f64(a), y = pilot_v_as_f64(b);
    switch (op) {
    case '+': return pilot_v_float(x + y);
    case '-': return pilot_v_float(x - y);
    case '*': return pilot_v_float(x * y);
    case '/': return pilot_v_float(x / y);
    default: return pilot_v_float(fmod(x, y));
    }
}
static inline pilot_val pilot_v_add(pilot_val a, pilot_val b) {
    if (a.tag == PILOT_V_STR || b.tag == PILOT_V_STR) return pilot_v_concat(a, b);
    return pilot_v_arith('+', a, b);
}
/* `& | ^ << >>` (`<` and `>` stand for the shifts). */
static inline pilot_val pilot_v_bits(char op, pilot_val a, pilot_val b) {
    const char *sym = op == '<' ? "<<" : op == '>' ? ">>" : op == '&' ? "&" : op == '|' ? "|" : "^";
    if (a.tag == PILOT_V_BOOL && b.tag == PILOT_V_BOOL && (op == '&' || op == '|' || op == '^')) {
        return pilot_v_bool(op == '&' ? (a.as.b && b.as.b) : op == '|' ? (a.as.b || b.as.b) : (a.as.b != b.as.b));
    }
    if (!pilot_v_is_intlike(a) || !pilot_v_is_intlike(b)) pilot_v_type_error(sym, a, b);
    int64_t x = a.as.i, y = b.as.i;
    int n = (int)(y & 63);
    switch (op) {
    case '&': return pilot_v_int(x & y);
    case '|': return pilot_v_int(x | y);
    case '^': return pilot_v_int(x ^ y);
    case '<': return pilot_v_int((int64_t)((uint64_t)x << n));
    default: return pilot_v_int(x >> n);
    }
}
static inline pilot_val pilot_v_pow(pilot_val a, pilot_val b) {
    if (pilot_v_is_intlike(a) && pilot_v_is_intlike(b)) {
        if (b.as.i < 0) return pilot_v_float(pow((double)a.as.i, (double)b.as.i));
        return pilot_v_int((int64_t)pilot_ipow(a.as.i, b.as.i));
    }
    return pilot_v_float(pow(pilot_v_to_f64(a), pilot_v_to_f64(b)));
}

/* ---- Comparison --------------------------------------------------------- */

static inline bool pilot_v_eq(pilot_val a, pilot_val b) {
    if (pilot_v_is_num(a) && pilot_v_is_num(b)) {
        if (pilot_v_is_intlike(a) && pilot_v_is_intlike(b)) return a.as.i == b.as.i;
        return pilot_v_as_f64(a) == pilot_v_as_f64(b);
    }
    if (a.tag != b.tag) return false;
    switch (a.tag) {
    case PILOT_V_NULL_TAG: return true;
    case PILOT_V_BOOL: return a.as.b == b.as.b;
    case PILOT_V_STR: return pilot_v_len(a) == pilot_v_len(b) && memcmp(a.as.s, b.as.s, pilot_v_len(a)) == 0;
    case PILOT_V_LIST:
        if (a.as.l == b.as.l) return true;
        if (a.as.l->len != b.as.l->len) return false;
        for (int64_t i = 0; i < a.as.l->len; i++) {
            if (!pilot_v_eq(a.as.l->items[i], b.as.l->items[i])) return false;
        }
        return true;
    case PILOT_V_REC: {
        if (a.as.r == b.as.r) return true;
        if (a.as.r->layout != b.as.r->layout) return false;
        int32_t n = pilot_v_rec_layout(a)->n;
        for (int32_t i = 0; i < n; i++) {
            if (!pilot_v_eq(pilot_v_rec_get(a, i), pilot_v_rec_get(b, i))) return false;
        }
        return true;
    }
    default: return a.as.m == b.as.m;
    }
}
/* -1, 0 or 1: numbers by value, strings by their bytes. */
static inline int pilot_v_order(pilot_val a, pilot_val b) {
    if (pilot_v_is_num(a) && pilot_v_is_num(b)) {
        if (pilot_v_is_intlike(a) && pilot_v_is_intlike(b)) return a.as.i < b.as.i ? -1 : a.as.i > b.as.i;
        double x = pilot_v_as_f64(a), y = pilot_v_as_f64(b);
        return x < y ? -1 : x > y ? 1 : 0;
    }
    if (a.tag == PILOT_V_STR && b.tag == PILOT_V_STR) {
        int c = pilot_v_cmp_text(&a, &b);
        return c < 0 ? -1 : c > 0;
    }
    pilot_v_type_error("<", a, b);
    return 0;
}
/* Ordering predicates: false when either side is NaN, as in C. */
static inline bool pilot_v_lt(pilot_val a, pilot_val b) {
    if (pilot_v_is_num(a) && pilot_v_is_num(b) && !(pilot_v_is_intlike(a) && pilot_v_is_intlike(b))) return pilot_v_as_f64(a) < pilot_v_as_f64(b);
    return pilot_v_order(a, b) < 0;
}
static inline bool pilot_v_le(pilot_val a, pilot_val b) {
    if (pilot_v_is_num(a) && pilot_v_is_num(b) && !(pilot_v_is_intlike(a) && pilot_v_is_intlike(b))) return pilot_v_as_f64(a) <= pilot_v_as_f64(b);
    return pilot_v_order(a, b) <= 0;
}

/* ---- Lists and records -------------------------------------------------- */

static inline pilot_val pilot_v_list_new(int64_t cap) {
    pilot_v_list *l = (pilot_v_list *)pilot_alloc(sizeof(pilot_v_list));
    l->len = 0;
    l->cap = cap > 0 ? cap : 4;
    l->items = (pilot_val *)pilot_alloc(sizeof(pilot_val) * (size_t)l->cap);
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_LIST;
    v.as.l = l;
    return v;
}
static inline void pilot_v_list_reserve(pilot_v_list *l, int64_t n) {
    if (n <= l->cap) return;
    int64_t cap = l->cap * 2;
    while (cap < n) cap *= 2;
    pilot_val *items = (pilot_val *)pilot_alloc(sizeof(pilot_val) * (size_t)cap);
    if (l->len) memcpy(items, l->items, sizeof(pilot_val) * (size_t)l->len);
    free(l->items);
    l->items = items;
    l->cap = cap;
}
/* A record (or function value) of `n` fields. */
static inline pilot_val pilot_v_record(int32_t n, const pilot_val *fields) {
    pilot_val r = pilot_v_list_new(n);
    if (n) memcpy(r.as.l->items, fields, sizeof(pilot_val) * (size_t)n);
    r.as.l->len = n;
    return r;
}
static inline pilot_v_list *pilot_v_need_list(pilot_val v, const char *what) {
    if (v.tag != PILOT_V_LIST) {
        char m[96];
        snprintf(m, sizeof m, "cannot %s %s", what, pilot_v_kind(v));
        pilot_trap(m);
    }
    return v.as.l;
}
static inline pilot_val pilot_v_field(pilot_val rec, int32_t i) {
    if (rec.tag == PILOT_V_REC) return pilot_v_rec_get(rec, i);
    pilot_v_list *l = pilot_v_need_list(rec, "read a field of");
    if (i < 0 || i >= l->len) pilot_trap("field index out of range");
    return l->items[i];
}
static inline void pilot_v_set_field(pilot_val rec, int32_t i, pilot_val v) {
    if (rec.tag == PILOT_V_REC) {
        const pilot_v_layout *rl = pilot_v_rec_layout(rec);
        if (i < 0 || i >= rl->n) pilot_trap("field index out of range");
        rl->set(rec, i, v);
        return;
    }
    pilot_v_list *l = pilot_v_need_list(rec, "write a field of");
    if (i < 0 || i >= l->len) pilot_trap("field index out of range");
    l->items[i] = v;
}
static inline int64_t pilot_v_index(pilot_v_list *l, pilot_val k) {
    char m[128];
    if (!pilot_v_is_intlike(k)) {
        snprintf(m, sizeof m, "list index must be an integer, found %s", pilot_v_kind(k));
        pilot_trap(m);
    }
    if (k.as.i < 0 || k.as.i >= l->len) {
        snprintf(m, sizeof m, "list index %" PRId64 " out of bounds for length %" PRId64, k.as.i, l->len);
        pilot_trap(m);
    }
    return k.as.i;
}

/* ---- Maps --------------------------------------------------------------- */

static inline pilot_val pilot_v_map_new(void) {
    pilot_v_map *m = (pilot_v_map *)pilot_alloc(sizeof(pilot_v_map));
    m->count = 0;
    m->used = 0;
    m->cap = 8;
    m->slots = (pilot_v_map_entry *)calloc((size_t)m->cap, sizeof(pilot_v_map_entry));
    if (!m->slots) pilot_trap("out of memory");
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_MAP;
    v.as.m = m;
    return v;
}
/* A key as text: a string, or the text the seed shows for it. */
static inline pilot_val pilot_v_key(pilot_val k) { return k.tag == PILOT_V_STR ? k : pilot_v_show(k); }
static inline uint64_t pilot_v_hash(pilot_val k) {
    uint64_t h = 1469598103934665603u;
    for (size_t i = 0, n = pilot_v_len(k); i < n; i++) {
        h ^= (unsigned char)k.as.s[i];
        h *= 1099511628211u;
    }
    return h;
}
/* The slot holding `key`, or the empty slot where it would go. */
static inline pilot_v_map_entry *pilot_v_map_slot(pilot_v_map *m, pilot_val key, uint64_t h, bool for_insert) {
    uint64_t mask = (uint64_t)m->cap - 1;
    pilot_v_map_entry *tomb = NULL;
    for (uint64_t i = h & mask;; i = (i + 1) & mask) {
        pilot_v_map_entry *e = &m->slots[i];
        if (e->state == 0) return (for_insert && tomb) ? tomb : e;
        if (e->state == 2) {
            if (!tomb) tomb = e;
            continue;
        }
        if (e->hash == h && pilot_v_len(e->key) == pilot_v_len(key) && memcmp(e->key.as.s, key.as.s, pilot_v_len(key)) == 0) return e;
    }
}
static inline void pilot_v_map_grow(pilot_v_map *m) {
    pilot_v_map_entry *old = m->slots;
    int64_t on = m->cap;
    m->cap *= 2;
    m->slots = (pilot_v_map_entry *)calloc((size_t)m->cap, sizeof(pilot_v_map_entry));
    if (!m->slots) pilot_trap("out of memory");
    m->used = m->count;
    for (int64_t i = 0; i < on; i++) {
        if (old[i].state != 1) continue;
        *pilot_v_map_slot(m, old[i].key, old[i].hash, true) = old[i];
    }
    free(old);
}
static inline pilot_v_map *pilot_v_need_map(pilot_val v, const char *what, pilot_val k) {
    if (v.tag != PILOT_V_MAP) {
        char m[160];
        pilot_val key = pilot_v_key(k);
        snprintf(m, sizeof m, "cannot %s `%.*s` %s %s", what, (int)pilot_v_len(key), key.as.s, what[0] == 'r' ? "from" : "on", pilot_v_kind(v));
        pilot_trap(m);
    }
    return v.as.m;
}
static inline pilot_val pilot_v_map_lookup(pilot_v_map *m, pilot_val k) {
    pilot_val key = pilot_v_key(k);
    pilot_v_map_entry *e = pilot_v_map_slot(m, key, pilot_v_hash(key), false);
    return e->state == 1 ? e->val : PILOT_V_NULL;
}
static inline void pilot_v_map_store(pilot_v_map *m, pilot_val k, pilot_val v) {
    pilot_val key = pilot_v_key(k);
    uint64_t h = pilot_v_hash(key);
    pilot_v_map_entry *e = pilot_v_map_slot(m, key, h, false);
    if (e->state == 1) {
        e->val = v;
        return;
    }
    if ((m->used + 1) * 4 > m->cap * 3) {
        pilot_v_map_grow(m);
    }
    e = pilot_v_map_slot(m, key, h, true);
    if (e->state == 0) m->used++;
    e->key = key;
    e->val = v;
    e->hash = h;
    e->state = 1;
    m->count++;
}

/* ---- Engine operations (`__engine` names, with their seed behaviour) ----- */

static inline pilot_val pilot_list_new(void) { return pilot_v_list_new(4); }
static inline pilot_val pilot_map_new(void) { return pilot_v_map_new(); }
static inline pilot_val pilot_is_null(pilot_val v) { return pilot_v_bool(v.tag == PILOT_V_NULL_TAG); }
static inline pilot_val pilot_to_str(pilot_val v) { return pilot_v_show(v); }

static inline pilot_val pilot_map_get(pilot_val o, pilot_val k) {
    if (o.tag == PILOT_V_LIST) return o.as.l->items[pilot_v_index(o.as.l, k)];
    if (o.tag == PILOT_V_STR && pilot_v_is_intlike(k)) {
        if (k.as.i >= 0 && k.as.i < (int64_t)pilot_v_len(o)) return pilot_v_int((unsigned char)o.as.s[k.as.i]);
        char m[128];
        snprintf(m, sizeof m, "string index %" PRId64 " out of bounds for length %" PRIu64, k.as.i, (uint64_t)pilot_v_len(o));
        pilot_trap(m);
    }
    return pilot_v_map_lookup(pilot_v_need_map(o, "read", k), k);
}
static inline pilot_val pilot_map_set(pilot_val o, pilot_val k, pilot_val v) {
    if (o.tag == PILOT_V_LIST) {
        o.as.l->items[pilot_v_index(o.as.l, k)] = v;
        return o;
    }
    pilot_v_map_store(pilot_v_need_map(o, "write", k), k, v);
    return o;
}
static inline pilot_val pilot_list_get(pilot_val o, pilot_val k) { return pilot_map_get(o, k); }
static inline pilot_val pilot_list_set(pilot_val o, pilot_val k, pilot_val v) { return pilot_map_set(o, k, v); }
static inline pilot_val pilot_map_has(pilot_val o, pilot_val k) {
    pilot_v_map *m = pilot_v_need_map(o, "read", k);
    pilot_val key = pilot_v_key(k);
    return pilot_v_bool(pilot_v_map_slot(m, key, pilot_v_hash(key), false)->state == 1);
}
static inline pilot_val pilot_map_remove(pilot_val o, pilot_val k) {
    pilot_v_map *m = pilot_v_need_map(o, "write", k);
    pilot_val key = pilot_v_key(k);
    pilot_v_map_entry *e = pilot_v_map_slot(m, key, pilot_v_hash(key), false);
    if (e->state != 1) return PILOT_V_NULL;
    e->state = 2;
    m->count--;
    return e->val;
}
static inline pilot_val pilot_list_push(pilot_val l, pilot_val x) {
    pilot_v_list *p = pilot_v_need_list(l, "push onto");
    pilot_v_list_reserve(p, p->len + 1);
    p->items[p->len++] = x;
    return l;
}
static inline pilot_val pilot_list_pop(pilot_val l) {
    pilot_v_list *p = pilot_v_need_list(l, "pop from");
    if (p->len == 0) pilot_trap("pop from an empty list");
    return p->items[--p->len];
}
static inline pilot_val pilot_str_len(pilot_val x) {
    if (x.tag == PILOT_V_STR) return pilot_v_int((int64_t)pilot_v_len(x));
    if (x.tag == PILOT_V_MAP) return pilot_v_int(x.as.m->count);
    return pilot_v_int(pilot_v_need_list(x, "take the length of")->len);
}
static inline pilot_val pilot_num_pow(pilot_val a, pilot_val b) { return pilot_v_pow(a, b); }
static inline void pilot_v_leaves(pilot_v_list *out, pilot_val v, bool deep) {
    pilot_v_list *l = pilot_v_need_list(v, "flatten");
    for (int64_t i = 0; i < l->len; i++) {
        pilot_val e = l->items[i];
        if (deep && e.tag == PILOT_V_LIST) {
            pilot_v_leaves(out, e, true);
        } else {
            pilot_v_list_reserve(out, out->len + 1);
            out->items[out->len++] = e;
        }
    }
}
static inline pilot_val pilot_list_extend(pilot_val l, pilot_val other, pilot_val deep) {
    pilot_val tmp = pilot_v_list_new(4);
    pilot_v_leaves(tmp.as.l, other, deep.tag == PILOT_V_BOOL && deep.as.b);
    pilot_v_list *p = pilot_v_need_list(l, "extend");
    pilot_v_list_reserve(p, p->len + tmp.as.l->len);
    for (int64_t i = 0; i < tmp.as.l->len; i++) p->items[p->len++] = tmp.as.l->items[i];
    return l;
}
static inline pilot_val pilot_list_remove_at(pilot_val l, pilot_val k) {
    pilot_v_list *p = pilot_v_need_list(l, "remove from");
    int64_t i = pilot_v_index(p, k);
    pilot_val out = p->items[i];
    memmove(p->items + i, p->items + i + 1, sizeof(pilot_val) * (size_t)(p->len - i - 1));
    p->len--;
    return out;
}
/* The first byte at or after `at` that starts a UTF-8 character. */
static inline int64_t pilot_v_boundary(pilot_val s, int64_t at) {
    int64_t len = (int64_t)pilot_v_len(s);
    int64_t u = at < 0 ? 0 : at > len ? len : at;
    while (u < len && ((unsigned char)s.as.s[u] & 0xC0) == 0x80) u++;
    return u;
}
static inline pilot_val pilot_str_slice(pilot_val v, pilot_val a, pilot_val b) {
    int64_t from = pilot_v_as_i64(a), to = pilot_v_as_i64(b);
    if (v.tag == PILOT_V_LIST) {
        int64_t n = v.as.l->len;
        int64_t x = from < 0 ? 0 : from > n ? n : from;
        int64_t y = to < 0 ? 0 : to > n ? n : to;
        if (y < x) y = x;
        return pilot_v_record((int32_t)(y - x), v.as.l->items + x);
    }
    if (v.tag != PILOT_V_STR) pilot_v_type_error("slice", v, b);
    int64_t x = pilot_v_boundary(v, from), y = pilot_v_boundary(v, to);
    if (y < x) y = x;
    return pilot_v_borrow(v.as.s + x, (size_t)(y - x));
}
static inline pilot_val pilot_str_byte_at(pilot_val s, pilot_val k) {
    int64_t i = pilot_v_as_i64(k);
    if (s.tag == PILOT_V_STR && i >= 0 && i < (int64_t)pilot_v_len(s)) return pilot_v_int((unsigned char)s.as.s[i]);
    char m[128];
    snprintf(m, sizeof m, "byte index %" PRId64 " out of bounds for length %" PRIu64, i, (uint64_t)(s.tag == PILOT_V_STR ? pilot_v_len(s) : 0));
    pilot_trap(m);
    return PILOT_V_NULL;
}
static inline pilot_val pilot_rt_assert(pilot_val cond, bool has_note, pilot_val note) {
    if (cond.tag == PILOT_V_BOOL && cond.as.b) return PILOT_V_NULL;
    if (!has_note) pilot_trap("assertion failed");
    pilot_val text = pilot_v_concat(PILOT_V_LIT("assertion failed: "), note);
    pilot_trap(text.as.s);
    return PILOT_V_NULL;
}
static inline pilot_val pilot_seam_open(pilot_val id, bool has_note, pilot_val note) {
    pilot_val text = pilot_v_concat(pilot_v_concat(PILOT_V_LIT("SEAM001: unconnected seam `"), id), PILOT_V_LIT("`: "));
    if (has_note) text = pilot_v_concat(text, note);
    pilot_trap(text.as.s);
    return PILOT_V_NULL;
}

/* ---- The process -------------------------------------------------------- */

static int pilot_argc;
static char **pilot_argv;

static inline void pilot_rt_init(int argc, char **argv) {
    pilot_argc = argc;
    pilot_argv = argv;
}
static inline pilot_val pilot_proc_args(void) {
    pilot_val l = pilot_v_list_new(pilot_argc);
    for (int i = 1; i < pilot_argc; i++) pilot_list_push(l, pilot_v_cstr(pilot_argv[i]));
    return l;
}

static pilot_v_buf pilot_stdin_text;
static size_t pilot_stdin_at;
static bool pilot_stdin_read;

/* The next line of standard input without its line break, or null at the
   end. */
static inline pilot_val pilot_stdin_read_line(void) {
    if (!pilot_stdin_read) {
        char chunk[65536];
        size_t n;
        while ((n = fread(chunk, 1, sizeof chunk, stdin)) > 0) pilot_v_buf_add(&pilot_stdin_text, chunk, n);
        pilot_stdin_read = true;
    }
    if (pilot_stdin_at >= pilot_stdin_text.len) return PILOT_V_NULL;
    const char *s = pilot_stdin_text.data + pilot_stdin_at;
    size_t rest = pilot_stdin_text.len - pilot_stdin_at;
    const char *nl = (const char *)memchr(s, '\n', rest);
    size_t len = nl ? (size_t)(nl - s) : rest;
    pilot_stdin_at += nl ? len + 1 : len;
    if (len > 0 && s[len - 1] == '\r') len--;
    return pilot_v_str(s, len);
}

/* ---- Function values ---------------------------------------------------- */

/* The module's dispatcher: function `index` of its table, called with the
   arguments then the captured values. */
typedef pilot_val (*pilot_v_dispatch)(int64_t index, int32_t argc, pilot_val *argv);

/* Calls function value `fv` (its index, then its captured values) with
   `argc` arguments through the module's dispatcher `call`. */
static inline pilot_val pilot_call_value(pilot_v_dispatch call, pilot_val fv, int32_t argc, const pilot_val *args) {
    pilot_v_list *l = pilot_v_need_list(fv, "call");
    if (l->len == 0 || !pilot_v_is_intlike(l->items[0])) pilot_trap("cannot call a value that is not a function");
    int32_t n = argc + (int32_t)(l->len - 1);
    pilot_val *all = (pilot_val *)pilot_alloc(sizeof(pilot_val) * (size_t)(n ? n : 1));
    if (argc) memcpy(all, args, sizeof(pilot_val) * (size_t)argc);
    if (l->len > 1) memcpy(all + argc, l->items + 1, sizeof(pilot_val) * (size_t)(l->len - 1));
    pilot_val out = call(l->items[0].as.i, n, all);
    free(all);
    return out;
}

static inline void pilot_arity(int32_t argc, int32_t want) {
    if (argc != want) {
        char m[96];
        snprintf(m, sizeof m, "a function of %d parameters called with %d arguments", want, argc);
        pilot_trap(m);
    }
}

#endif /* __cplusplus */

#endif /* PILOT_RT_H */
