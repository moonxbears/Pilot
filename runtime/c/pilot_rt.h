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
 * `pilot_val`: a tag, a string's header and first 4 bytes, and a payload. A struct value is a
 * C struct the emitted module declares (see "Records"); other records and
 * function values are lists of their fields, a function value's field 0
 * being its index in the module's function table. This is the registered
 * stand-in `emit.c_dynamic_values`: the design lays every value out by its
 * physics and holds value types in place. Strings are in
 * runtime/c/pilot_rt_str.h.
 *
 * Memory. Strings, lists, maps and struct values are reference counted
 * (F7, see "Ownership"): the emitted code releases a value when the last
 * reference to it goes, and the object is freed then. Compiled with
 * PILOT_LIVE_COUNT, a program reports on standard error, when it ends
 * normally, how many objects are still allocated (`live allocations: N`);
 * PILOT_NO_FREE turns freeing off, so that count is every object made.
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
#include <stddef.h>
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
    /* Byte 0 is the tag; bytes 1..3 a string's header (see "Strings"). */
    uint32_t tag : 8;
    uint32_t hdr : 24;
    /* A string's first 4 bytes, in memory order; 0 otherwise. */
    uint32_t pre;
    union {
        int64_t i;
        uint64_t u;
        double f;
        bool b;
        /* A string of up to 12 bytes holds its bytes 4..11 here. */
        struct pilot_v_list *l;
        struct pilot_v_map *m;
        struct pilot_v_rec *r;
    } as;
} pilot_val;

/* The count of a list laid out as static data (a constant): never reaches
   0, so it is never freed. */
#define PILOT_RC_STATIC (INT64_MAX / 2)

typedef struct pilot_v_list {
    int64_t rc;
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
    int64_t rc;
    int64_t count;
    int64_t used;
    int64_t cap;
    pilot_v_map_entry *slots;
} pilot_v_map;

/* A function value's code: the arguments, then the captured values. */
typedef pilot_val (*pilot_v_fn)(int32_t argc, pilot_val *argv);

#define PILOT_V_NULL ((pilot_val){0})
/*
 * Ownership. Every entry point below borrows its arguments and gives an
 * owned result: one reference the caller must release (`pilot_v_drop`) or
 * hand on. A runtime operation that returns a value it also keeps (a list
 * element, the list `push` returns) takes a new reference for the caller
 * (`pilot_v_dup`); storing a value into a list, map or struct takes a
 * reference for the container and releases the value it replaces. A field
 * or element read through `pilot_v_field` or a layout's `get` is borrowed.
 * Objects are freed when their count reaches 0. A string literal is a
 * static object whose count is 0: it is never counted or freed.
 */
#ifdef PILOT_LIVE_COUNT
static int64_t pilot_live;
#define PILOT_LIVE(d) (pilot_live += (d))
#else
#define PILOT_LIVE(d) ((void)0)
#endif

/* Reports the objects still allocated, when the program counts them. */
static inline void pilot_rt_finish(void) {
#ifdef PILOT_LIVE_COUNT
    fflush(stdout);
    fprintf(stderr, "live allocations: %" PRId64 "\n", pilot_live);
#endif
}

static inline void *pilot_alloc(size_t size) {
    void *p = malloc(size ? size : 1);
    if (!p) pilot_trap("out of memory");
    return p;
}

/* Strings: runtime/c/pilot_rt_str.h. */
#include "pilot_rt_str.h"

/*
 * Small fixed-size objects (struct values): carved in 16-byte steps from
 * 64 KiB chunks rather than one malloc each, and kept on a free list per
 * size when freed, for the next object of that size.
 */
#define PILOT_OBJ_CHUNK ((size_t)64 * 1024)
#define PILOT_OBJ_CLASSES (PILOT_OBJ_CHUNK / 4 / 16 + 1)
static char *pilot_obj_at, *pilot_obj_end;
static void *pilot_obj_free[PILOT_OBJ_CLASSES];
static inline void *pilot_alloc_obj(size_t size) {
    size = (size + 15) & ~(size_t)15;
    if (size > PILOT_OBJ_CHUNK / 4) return pilot_alloc(size);
    void **head = &pilot_obj_free[size / 16];
    if (*head) {
        void *p = *head;
        *head = *(void **)p;
        return p;
    }
    if ((size_t)(pilot_obj_end - pilot_obj_at) < size) {
        pilot_obj_at = (char *)pilot_alloc(PILOT_OBJ_CHUNK);
        pilot_obj_end = pilot_obj_at + PILOT_OBJ_CHUNK;
    }
    void *p = pilot_obj_at;
    pilot_obj_at += size;
    return p;
}
static inline void pilot_free_obj(void *p, size_t size) {
    size = (size + 15) & ~(size_t)15;
    if (size > PILOT_OBJ_CHUNK / 4) {
        free(p);
        return;
    }
    *(void **)p = pilot_obj_free[size / 16];
    pilot_obj_free[size / 16] = p;
}

/* Takes one more reference to `v`. */
static inline void pilot_v_dup(pilot_val v);
/* Releases one reference to `v`, freeing it when it was the last. */
static inline void pilot_v_drop(pilot_val v);

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
    uint32_t rc;
} pilot_v_rec;

/* A layout's field count, its dynamic field reader (borrowed) and writer,
   what releasing a value of it releases (its fields), its size, its
   alignment (above 16 bytes for a struct written `[Align(N)]`) and its
   fields' names, in field order (for printing a value). */
typedef struct pilot_v_layout {
    int32_t n;
    pilot_val (*get)(pilot_val rec, int32_t i);
    void (*set)(pilot_val rec, int32_t i, pilot_val v);
    void (*drop)(pilot_v_rec *rec);
    size_t size;
    size_t align;
    const char *const *names;
} pilot_v_layout;

/* Defined by the emitted module when it has record layouts. */
static const pilot_v_layout *pilot_rt_layouts;

/* A new struct value of `size` bytes and layout `layout`, one reference. */
/* Records aligned above what the object pool gives (16 bytes) come from the
   platform's aligned allocator and go back to it. */
static inline size_t pilot_v_rec_align(uint32_t layout) {
    return pilot_rt_layouts ? pilot_rt_layouts[layout].align : 0;
}
static inline void *pilot_alloc_aligned(size_t align, size_t size) {
    size = (size + align - 1) & ~(align - 1);
#if defined(_WIN32)
    void *p = _aligned_malloc(size, align);
#else
    void *p = aligned_alloc(align, size);
#endif
    if (!p) pilot_trap("out of memory");
    return p;
}
static inline void pilot_free_aligned(void *p) {
#if defined(_WIN32)
    _aligned_free(p);
#else
    free(p);
#endif
}
static inline pilot_v_rec *pilot_v_rec_new(size_t size, uint32_t layout) {
    size_t align = pilot_v_rec_align(layout);
    pilot_v_rec *h = (pilot_v_rec *)(align > 16 ? pilot_alloc_aligned(align, size) : pilot_alloc_obj(size));
    h->layout = layout;
    h->rc = 1;
    PILOT_LIVE(1);
    return h;
}
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
    return pilot_v_str_cmp(*x, *y);
}

static inline pilot_val pilot_v_show(pilot_val v) {
    if (v.tag == PILOT_V_STR) {
        pilot_v_dup(v);
        return v;
    }
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
    case PILOT_V_STR: pilot_v_buf_add(b, pilot_v_bytes(&v), pilot_v_len(v)); return;
    case PILOT_V_LIST:
        pilot_v_buf_cstr(b, "[");
        for (int64_t i = 0; i < v.as.l->len; i++) {
            if (i) pilot_v_buf_cstr(b, ", ");
            pilot_v_show_into(b, v.as.l->items[i]);
        }
        pilot_v_buf_cstr(b, "]");
        return;
    case PILOT_V_REC: {
        /* As the seed shows a struct: `{ name: value, .. }`, the entries'
           texts in order, as a map is shown. */
        const pilot_v_layout *l = pilot_v_rec_layout(v);
        pilot_val *parts = (pilot_val *)pilot_alloc(sizeof(pilot_val) * (size_t)(l->n ? l->n : 1));
        for (int32_t i = 0; i < l->n; i++) {
            pilot_v_buf e = {0};
            pilot_v_buf_cstr(&e, l->names[i]);
            pilot_v_buf_cstr(&e, ": ");
            pilot_v_show_into(&e, pilot_v_rec_get(v, i));
            parts[i] = pilot_v_str(e.data ? e.data : "", e.len);
            free(e.data);
        }
        qsort(parts, (size_t)l->n, sizeof(pilot_val), pilot_v_cmp_text);
        pilot_v_buf_cstr(b, "{ ");
        for (int32_t i = 0; i < l->n; i++) {
            if (i) pilot_v_buf_cstr(b, ", ");
            pilot_v_buf_add(b, pilot_v_bytes(&parts[i]), pilot_v_len(parts[i]));
            pilot_v_drop(parts[i]);
        }
        pilot_v_buf_cstr(b, " }");
        free(parts);
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
            pilot_v_buf_add(&e, pilot_v_bytes(&m->slots[i].key), pilot_v_len(m->slots[i].key));
            pilot_v_buf_cstr(&e, ": ");
            pilot_v_show_into(&e, m->slots[i].val);
            parts[n++] = pilot_v_str(e.data ? e.data : "", e.len);
            free(e.data);
        }
        qsort(parts, (size_t)n, sizeof(pilot_val), pilot_v_cmp_text);
        pilot_v_buf_cstr(b, "{ ");
        for (int64_t i = 0; i < n; i++) {
            if (i) pilot_v_buf_cstr(b, ", ");
            pilot_v_buf_add(b, pilot_v_bytes(&parts[i]), pilot_v_len(parts[i]));
            pilot_v_drop(parts[i]);
        }
        pilot_v_buf_cstr(b, " }");
        free(parts);
        return;
    }
    }
}

static inline void pilot_v_put(FILE *f, pilot_val v) {
    if (v.tag == PILOT_V_STR) {
        fwrite(pilot_v_bytes(&v), 1, pilot_v_len(v), f);
        return;
    }
    pilot_val s = pilot_v_show(v);
    fwrite(pilot_v_bytes(&s), 1, pilot_v_len(s), f);
    pilot_v_drop(s);
}

/*
 * Appending in place. Strings are immutable, so `a + b` may write `b` just
 * past `a` when `a` ends where its buffer's written bytes end: every other
 * string in that buffer is a prefix of `a`, and its bytes do not change.
 * The result then shares `a`'s buffer. A buffer concatenation made has room
 * for appending (`cap`); one that runs out of room is replaced by one twice
 * as large, so a loop of `s = s + x` copies O(n) bytes in all rather than
 * O(n^2). Only strings whose length is kept in the value take part (a long
 * string's length lives in its buffer, which the strings sharing it read),
 * and never a literal.
 */
static inline pilot_val pilot_v_concat(pilot_val a, pilot_val b) {
    bool shown_a = a.tag != PILOT_V_STR, shown_b = b.tag != PILOT_V_STR;
    if (shown_a) a = pilot_v_show(a);
    if (shown_b) b = pilot_v_show(b);
    size_t an = pilot_v_len(a), bn = pilot_v_len(b), n = an + bn;
    bool high = pilot_v_non_ascii(a) || pilot_v_non_ascii(b);
    const char *as = pilot_v_bytes(&a), *bs = pilot_v_bytes(&b);
    pilot_val v = PILOT_V_NULL;
    if (n <= PILOT_V_INLINE_MAX) {
        char tmp[16] = {0};
        memcpy(tmp, as, an);
        memcpy(tmp + an, bs, bn);
        v = pilot_v_str_inline(tmp, n, high);
    } else {
        pilot_v_strhdr *h = pilot_v_counted(a) && pilot_v_hdr(a) < PILOT_V_HDR_LONG ? pilot_v_strhead(as) : NULL;
        bool at_end = h && h->cap != 0 && h->fill == an;
        if (at_end && n <= h->cap && n <= PILOT_V_SHORT_MAX) {
            char *at = (char *)as;
            memcpy(at + an, bs, bn);
            at[n] = 0;
            h->fill = n;
            h->rc++;
            pilot_v_str_point(&v, at, n, high, true);
        } else {
            /* A string that keeps growing gets twice the room it needs. */
            size_t room = n > PILOT_V_SHORT_MAX ? 0 : at_end ? n * 2 : n;
            if (room > PILOT_V_SHORT_MAX) room = PILOT_V_SHORT_MAX;
            char *s = pilot_v_strroom(n, room);
            memcpy(s, as, an);
            memcpy(s + an, bs, bn);
            pilot_v_str_point(&v, s, n, high, true);
        }
    }
    if (shown_a) pilot_v_drop(a);
    if (shown_b) pilot_v_drop(b);
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
    case PILOT_V_STR: return pilot_v_str_eq(a, b);
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
        return pilot_v_str_cmp(a, b);
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
    l->rc = 1;
    l->len = 0;
    l->cap = cap > 0 ? cap : 4;
    l->items = (pilot_val *)pilot_alloc(sizeof(pilot_val) * (size_t)l->cap);
    PILOT_LIVE(1);
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
    for (int32_t i = 0; i < n; i++) {
        pilot_v_dup(fields[i]);
        r.as.l->items[i] = fields[i];
    }
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
/* Field `i` of a record, borrowed. */
static inline pilot_val pilot_v_field(pilot_val rec, int32_t i) {
    if (rec.tag == PILOT_V_REC) return pilot_v_rec_get(rec, i);
    pilot_v_list *l = pilot_v_need_list(rec, "read a field of");
    if (i < 0 || i >= l->len) pilot_trap("field index out of range");
    return l->items[i];
}
/* Replaces `*cell` with `v`: a reference to `v`, the old value released. */
static inline void pilot_v_replace(pilot_val *cell, pilot_val v) {
    pilot_v_dup(v);
    pilot_v_drop(*cell);
    *cell = v;
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
    pilot_v_replace(&l->items[i], v);
}
/* Stops the program: `k` is not an index of list `l`. */
static PILOT_OUTLINE void pilot_v_index_error(pilot_v_list *l, pilot_val k) {
    char m[128];
    if (!pilot_v_is_intlike(k)) snprintf(m, sizeof m, "list index must be an integer, found %s", pilot_v_kind(k));
    else snprintf(m, sizeof m, "list index %" PRId64 " out of bounds for length %" PRId64, k.as.i, l->len);
    pilot_trap(m);
}
static inline int64_t pilot_v_index(pilot_v_list *l, pilot_val k) {
    if (!pilot_v_is_intlike(k) || k.as.i < 0 || k.as.i >= l->len) pilot_v_index_error(l, k);
    return k.as.i;
}

/* ---- Maps --------------------------------------------------------------- */

static inline pilot_val pilot_v_map_new(void) {
    pilot_v_map *m = (pilot_v_map *)pilot_alloc(sizeof(pilot_v_map));
    m->rc = 1;
    m->count = 0;
    m->used = 0;
    m->cap = 8;
    m->slots = (pilot_v_map_entry *)calloc((size_t)m->cap, sizeof(pilot_v_map_entry));
    if (!m->slots) pilot_trap("out of memory");
    PILOT_LIVE(1);
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_MAP;
    v.as.m = m;
    return v;
}
/* A key as text: a string, or the text the seed shows for it; owned. */
static inline pilot_val pilot_v_key(pilot_val k) { return pilot_v_show(k); }
static inline uint64_t pilot_v_hash(pilot_val k) {
    return pilot_hash_bytes(pilot_v_bytes(&k), pilot_v_len(k));
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
        if (e->hash == h && pilot_v_str_eq(e->key, key)) return e;
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
        snprintf(m, sizeof m, "cannot %s `%.*s` %s %s", what, (int)pilot_v_len(key), pilot_v_bytes(&key), what[0] == 'r' ? "from" : "on", pilot_v_kind(v));
        pilot_trap(m);
    }
    return v.as.m;
}
/* The entry of key `k`, or the empty slot where it would go. */
static inline pilot_v_map_entry *pilot_v_map_find(pilot_v_map *m, pilot_val k) {
    pilot_val key = pilot_v_key(k);
    pilot_v_map_entry *e = pilot_v_map_slot(m, key, pilot_v_hash(key), false);
    pilot_v_drop(key);
    return e;
}
/* The value of key `k`, borrowed; null when there is none. */
static inline pilot_val pilot_v_map_lookup(pilot_v_map *m, pilot_val k) {
    pilot_v_map_entry *e = pilot_v_map_find(m, k);
    return e->state == 1 ? e->val : PILOT_V_NULL;
}
static inline void pilot_v_map_store(pilot_v_map *m, pilot_val k, pilot_val v) {
    pilot_val key = pilot_v_key(k);
    uint64_t h = pilot_v_hash(key);
    pilot_v_map_entry *e = pilot_v_map_slot(m, key, h, false);
    if (e->state == 1) {
        pilot_v_drop(key);
        pilot_v_replace(&e->val, v);
        return;
    }
    if ((m->used + 1) * 4 > m->cap * 3) {
        pilot_v_map_grow(m);
    }
    e = pilot_v_map_slot(m, key, h, true);
    if (e->state == 0) m->used++;
    e->key = key;
    pilot_v_dup(v);
    e->val = v;
    e->hash = h;
    e->state = 1;
    m->count++;
}

/* ---- Releasing ---------------------------------------------------------- */

static void pilot_v_free(pilot_val v) {
    switch (v.tag) {
    case PILOT_V_STR: free(pilot_v_strhead(pilot_v_ptr(v))); break;
    case PILOT_V_LIST: {
        pilot_v_list *l = v.as.l;
        for (int64_t i = 0; i < l->len; i++) pilot_v_drop(l->items[i]);
        free(l->items);
        free(l);
        break;
    }
    case PILOT_V_MAP: {
        pilot_v_map *m = v.as.m;
        for (int64_t i = 0; i < m->cap; i++) {
            if (m->slots[i].state != 1) continue;
            pilot_v_drop(m->slots[i].key);
            pilot_v_drop(m->slots[i].val);
        }
        free(m->slots);
        free(m);
        break;
    }
    default: {
        const pilot_v_layout *l = pilot_v_rec_layout(v);
        l->drop(v.as.r);
        if (l->align > 16) pilot_free_aligned(v.as.r);
        else pilot_free_obj(v.as.r, l->size);
        break;
    }
    }
    PILOT_LIVE(-1);
}
static inline void pilot_v_dup(pilot_val v) {
    switch (v.tag) {
    case PILOT_V_STR:
        if (pilot_v_counted(v)) pilot_v_strhead(pilot_v_ptr(v))->rc++;
        return;
    case PILOT_V_LIST: v.as.l->rc++; return;
    case PILOT_V_MAP: v.as.m->rc++; return;
    case PILOT_V_REC: v.as.r->rc++; return;
    default: return;
    }
}
static inline void pilot_v_drop(pilot_val v) {
#ifdef PILOT_NO_FREE
    (void)v;
#else
    switch (v.tag) {
    case PILOT_V_STR:
        if (pilot_v_counted(v) && --pilot_v_strhead(pilot_v_ptr(v))->rc == 0) pilot_v_free(v);
        return;
    case PILOT_V_LIST: if (--v.as.l->rc == 0) pilot_v_free(v); return;
    case PILOT_V_MAP: if (--v.as.m->rc == 0) pilot_v_free(v); return;
    case PILOT_V_REC: if (--v.as.r->rc == 0) pilot_v_free(v); return;
    default: return;
    }
#endif
}

/* ---- Engine operations (`__engine` names, with their seed behaviour) ----- */

static inline pilot_val pilot_list_new(void) { return pilot_v_list_new(4); }
static inline pilot_val pilot_map_new(void) { return pilot_v_map_new(); }
static inline pilot_val pilot_is_null(pilot_val v) { return pilot_v_bool(v.tag == PILOT_V_NULL_TAG); }
static inline pilot_val pilot_to_str(pilot_val v) { return pilot_v_show(v); }

/* `o[k]` for a string or map `o`: the slow paths of `pilot_map_get`. */
static PILOT_OUTLINE pilot_val pilot_v_get_other(pilot_val o, pilot_val k) {
    if (o.tag == PILOT_V_STR && pilot_v_is_intlike(k)) {
        if (k.as.i >= 0 && k.as.i < (int64_t)pilot_v_len(o)) return pilot_v_int((unsigned char)pilot_v_bytes(&o)[k.as.i]);
        char m[128];
        snprintf(m, sizeof m, "string index %" PRId64 " out of bounds for length %" PRIu64, k.as.i, (uint64_t)pilot_v_len(o));
        pilot_trap(m);
        return PILOT_V_NULL;
    }
    pilot_val out = pilot_v_map_lookup(pilot_v_need_map(o, "read", k), k);
    pilot_v_dup(out);
    return out;
}
static inline pilot_val pilot_map_get(pilot_val o, pilot_val k) {
    if (o.tag != PILOT_V_LIST) return pilot_v_get_other(o, k);
    pilot_val out = o.as.l->items[pilot_v_index(o.as.l, k)];
    pilot_v_dup(out);
    return out;
}
static inline pilot_val pilot_map_set(pilot_val o, pilot_val k, pilot_val v) {
    if (o.tag == PILOT_V_LIST) {
        pilot_v_replace(&o.as.l->items[pilot_v_index(o.as.l, k)], v);
    } else {
        pilot_v_map_store(pilot_v_need_map(o, "write", k), k, v);
    }
    pilot_v_dup(o);
    return o;
}
static inline pilot_val pilot_list_get(pilot_val o, pilot_val k) { return pilot_map_get(o, k); }
static inline pilot_val pilot_list_set(pilot_val o, pilot_val k, pilot_val v) { return pilot_map_set(o, k, v); }
static inline pilot_val pilot_map_has(pilot_val o, pilot_val k) {
    pilot_v_map *m = pilot_v_need_map(o, "read", k);
    return pilot_v_bool(pilot_v_map_find(m, k)->state == 1);
}
/* The removed value: the map's reference passes to the caller. */
static inline pilot_val pilot_map_remove(pilot_val o, pilot_val k) {
    pilot_v_map *m = pilot_v_need_map(o, "write", k);
    pilot_v_map_entry *e = pilot_v_map_find(m, k);
    if (e->state != 1) return PILOT_V_NULL;
    e->state = 2;
    m->count--;
    pilot_v_drop(e->key);
    pilot_val out = e->val;
    e->key = PILOT_V_NULL;
    e->val = PILOT_V_NULL;
    return out;
}
static inline pilot_val pilot_list_push(pilot_val l, pilot_val x) {
    pilot_v_list *p = pilot_v_need_list(l, "push onto");
    pilot_v_list_reserve(p, p->len + 1);
    pilot_v_dup(x);
    p->items[p->len++] = x;
    pilot_v_dup(l);
    return l;
}
static inline int pilot_v_cmp_entry_key(const void *a, const void *b) {
    const pilot_v_map_entry *x = *(const pilot_v_map_entry *const *)a;
    const pilot_v_map_entry *y = *(const pilot_v_map_entry *const *)b;
    return pilot_v_cmp_text(&x->key, &y->key);
}
/* `map_values(m)`: a new list of the map's values in the order of their
   keys (the order a map is shown in); the list holds its own references. */
static inline pilot_val pilot_map_values(pilot_val o) {
    pilot_v_map *m = pilot_v_need_map(o, "read", PILOT_V_NULL);
    pilot_v_map_entry **es = (pilot_v_map_entry **)pilot_alloc(sizeof(pilot_v_map_entry *) * (size_t)(m->count ? m->count : 1));
    int64_t n = 0;
    for (int64_t i = 0; i < m->cap; i++) {
        if (m->slots[i].state == 1) es[n++] = &m->slots[i];
    }
    qsort(es, (size_t)n, sizeof(pilot_v_map_entry *), pilot_v_cmp_entry_key);
    pilot_val out = pilot_v_list_new(n ? n : 1);
    pilot_v_list *p = out.as.l;
    for (int64_t i = 0; i < n; i++) {
        pilot_v_dup(es[i]->val);
        p->items[p->len++] = es[i]->val;
    }
    free(es);
    return out;
}
/* `value_clone(v)`: a deep copy of `v` that shares nothing that could
   change: lists, maps and struct values are copied with their contents
   copied the same way; strings, which never change, and scalars are shared.
   A constant's copy (`var x = CONST;`) is made this way. `v` is borrowed. */
static PILOT_OUTLINE pilot_val pilot_v_clone(pilot_val v, int depth) {
    if (depth > 4096) pilot_trap("a value nests too deeply to copy");
    switch (v.tag) {
    case PILOT_V_LIST: {
        pilot_v_list *l = v.as.l;
        pilot_val out = pilot_v_list_new(l->len);
        for (int64_t i = 0; i < l->len; i++) out.as.l->items[i] = pilot_v_clone(l->items[i], depth + 1);
        out.as.l->len = l->len;
        return out;
    }
    case PILOT_V_MAP: {
        pilot_v_map *m = v.as.m;
        pilot_val out = pilot_v_map_new();
        for (int64_t i = 0; i < m->cap; i++) {
            if (m->slots[i].state != 1) continue;
            pilot_val c = pilot_v_clone(m->slots[i].val, depth + 1);
            pilot_v_map_store(out.as.m, m->slots[i].key, c);
            pilot_v_drop(c);
        }
        return out;
    }
    case PILOT_V_REC: {
        const pilot_v_layout *rl = pilot_v_rec_layout(v);
        pilot_v_rec *h = pilot_v_rec_new(rl->size, v.as.r->layout);
        memcpy((char *)h + sizeof(pilot_v_rec), (const char *)v.as.r + sizeof(pilot_v_rec), rl->size - sizeof(pilot_v_rec));
        pilot_val out = pilot_v_rec_val(h);
        for (int32_t i = 0; i < rl->n; i++) {
            /* The copied bytes hold the original's references: take one of
               our own, then replace the field with its copy. */
            pilot_val f = rl->get(out, i);
            pilot_v_dup(f);
            pilot_val c = pilot_v_clone(f, depth + 1);
            rl->set(out, i, c);
            pilot_v_drop(c);
        }
        return out;
    }
    default:
        pilot_v_dup(v);
        return v;
    }
}
static inline pilot_val pilot_value_clone(pilot_val v) { return pilot_v_clone(v, 0); }
/* The popped element: the list's reference passes to the caller. */
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
/* Appends the elements of list `v` (its leaves, when `deep`) to `out`,
   borrowed. */
static inline void pilot_v_leaves(pilot_v_buf *out, pilot_val v, bool deep) {
    pilot_v_list *l = pilot_v_need_list(v, "flatten");
    for (int64_t i = 0; i < l->len; i++) {
        pilot_val e = l->items[i];
        if (deep && e.tag == PILOT_V_LIST) {
            pilot_v_leaves(out, e, true);
        } else {
            pilot_v_buf_add(out, (const char *)&e, sizeof e);
        }
    }
}
static inline pilot_val pilot_list_extend(pilot_val l, pilot_val other, pilot_val deep) {
    pilot_v_buf tmp = {0};
    pilot_v_leaves(&tmp, other, deep.tag == PILOT_V_BOOL && deep.as.b);
    pilot_v_list *p = pilot_v_need_list(l, "extend");
    int64_t n = (int64_t)(tmp.len / sizeof(pilot_val));
    pilot_v_list_reserve(p, p->len + n);
    for (int64_t i = 0; i < n; i++) {
        pilot_val e;
        memcpy(&e, tmp.data + (size_t)i * sizeof e, sizeof e);
        pilot_v_dup(e);
        p->items[p->len++] = e;
    }
    free(tmp.data);
    pilot_v_dup(l);
    return l;
}
/* The removed element: the list's reference passes to the caller. */
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
    const char *b = pilot_v_bytes(&s);
    while (u < len && ((unsigned char)b[u] & 0xC0) == 0x80) u++;
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
    /* A copy: a string's bytes start its buffer. */
    return pilot_v_str(pilot_v_bytes(&v) + x, (size_t)(y - x));
}
/* `h.find(n)` / `h.find(n, start)`: the byte offset of the first `n` in `h`
   at or after `start` (moved forward to a character boundary), or -1. */
static inline pilot_val pilot_str_find(pilot_val h, pilot_val n, pilot_val start) {
    if (h.tag != PILOT_V_STR || n.tag != PILOT_V_STR) pilot_v_type_error("find", h, n);
    int64_t from = pilot_v_is_num(start) ? pilot_v_as_i64(start) : 0;
    if (from < 0) return pilot_v_int(-1);
    int64_t len = (int64_t)pilot_v_len(h), m = (int64_t)pilot_v_len(n);
    int64_t u = pilot_v_boundary(h, from);
    if (m == 0) return pilot_v_int(u);
    /* A needle with a byte 0x80 or above is never in an ASCII text. */
    if (pilot_v_non_ascii(n) && !pilot_v_non_ascii(h)) return pilot_v_int(-1);
    return pilot_v_int(pilot_find_bytes(pilot_v_bytes(&h), (size_t)len, pilot_v_bytes(&n), (size_t)m, (size_t)u));
}
/* Stops the program: argument `i` (from 1) of `op` is not a string. */
static inline void pilot_v_need_str(const char *op, int i, pilot_val v) {
    if (v.tag == PILOT_V_STR) return;
    char m[160];
    snprintf(m, sizeof m, "`%s` argument %d must be a string, found %s", op, i, pilot_v_kind(v));
    pilot_trap(m);
}
/* `contains`: an element of a list, a key of a map, or text in a string. */
static inline pilot_val pilot_str_contains(pilot_val h, pilot_val n) {
    if (h.tag == PILOT_V_LIST) {
        for (int64_t i = 0; i < h.as.l->len; i++) {
            if (pilot_v_eq(h.as.l->items[i], n)) return pilot_v_bool(true);
        }
        return pilot_v_bool(false);
    }
    if (h.tag == PILOT_V_MAP) return pilot_map_has(h, n);
    pilot_v_need_str("str_contains", 1, h);
    pilot_v_need_str("str_contains", 2, n);
    size_t m = pilot_v_len(n);
    if (m && pilot_v_non_ascii(n) && !pilot_v_non_ascii(h)) return pilot_v_bool(false);
    return pilot_v_bool(pilot_find_bytes(pilot_v_bytes(&h), pilot_v_len(h), pilot_v_bytes(&n), m, 0) >= 0);
}
static inline pilot_val pilot_str_starts_with(pilot_val h, pilot_val n) {
    pilot_v_need_str("str_starts_with", 1, h);
    pilot_v_need_str("str_starts_with", 2, n);
    size_t hn = pilot_v_len(h), m = pilot_v_len(n);
    return pilot_v_bool(m <= hn && memcmp(pilot_v_bytes(&h), pilot_v_bytes(&n), m) == 0);
}
static inline pilot_val pilot_str_ends_with(pilot_val h, pilot_val n) {
    pilot_v_need_str("str_ends_with", 1, h);
    pilot_v_need_str("str_ends_with", 2, n);
    size_t hn = pilot_v_len(h), m = pilot_v_len(n);
    return pilot_v_bool(m <= hn && memcmp(pilot_v_bytes(&h) + hn - m, pilot_v_bytes(&n), m) == 0);
}
/* Whether code point `c` is Unicode White_Space (what `trim` removes). */
static inline bool pilot_v_white(uint32_t c) {
    return (c >= 9 && c <= 13) || c == 32 || c == 0x85 || c == 0xA0 || c == 0x1680 || (c >= 0x2000 && c <= 0x200A)
        || c == 0x2028 || c == 0x2029 || c == 0x202F || c == 0x205F || c == 0x3000;
}
/* The code point of the UTF-8 character at `s[i]` and its length. */
static inline uint32_t pilot_v_decode(const unsigned char *s, size_t i, size_t *len) {
    unsigned char b = s[i];
    if (b < 0x80) { *len = 1; return b; }
    if (b < 0xE0) { *len = 2; return ((uint32_t)(b & 0x1F) << 6) | (s[i + 1] & 0x3F); }
    if (b < 0xF0) { *len = 3; return ((uint32_t)(b & 0x0F) << 12) | ((uint32_t)(s[i + 1] & 0x3F) << 6) | (s[i + 2] & 0x3F); }
    *len = 4;
    return ((uint32_t)(b & 0x07) << 18) | ((uint32_t)(s[i + 1] & 0x3F) << 12) | ((uint32_t)(s[i + 2] & 0x3F) << 6) | (s[i + 3] & 0x3F);
}
/* `trim`: the string without the white space at either end. */
static inline pilot_val pilot_str_trim(pilot_val v) {
    pilot_v_need_str("str_trim", 1, v);
    const unsigned char *s = (const unsigned char *)pilot_v_bytes(&v);
    size_t n = pilot_v_len(v), a = 0, b = n, len;
    while (a < b && pilot_v_white(pilot_v_decode(s, a, &len))) a += len;
    while (b > a) {
        size_t c = b - 1;
        while (c > a && (s[c] & 0xC0) == 0x80) c--;
        if (!pilot_v_white(pilot_v_decode(s, c, &len))) break;
        b = c;
    }
    if (a == 0 && b == n) {
        pilot_v_dup(v);
        return v;
    }
    return pilot_v_str((const char *)s + a, b - a);
}
/* `replace`: every `from` in `h`, left to right without overlaps, made
   `to`; an empty `from` changes nothing. */
static inline pilot_val pilot_str_replace(pilot_val h, pilot_val from, pilot_val to) {
    pilot_v_need_str("str_replace", 1, h);
    pilot_v_need_str("str_replace", 2, from);
    pilot_v_need_str("str_replace", 3, to);
    size_t hn = pilot_v_len(h), fn = pilot_v_len(from), tn = pilot_v_len(to);
    const char *hs = pilot_v_bytes(&h), *fs = pilot_v_bytes(&from), *ts = pilot_v_bytes(&to);
    int64_t at = fn == 0 || (pilot_v_non_ascii(from) && !pilot_v_non_ascii(h)) ? -1 : pilot_find_bytes(hs, hn, fs, fn, 0);
    if (at < 0) {
        pilot_v_dup(h);
        return h;
    }
    pilot_v_buf b = {0};
    size_t done = 0;
    while (at >= 0) {
        pilot_v_buf_add(&b, hs + done, (size_t)at - done);
        pilot_v_buf_add(&b, ts, tn);
        done = (size_t)at + fn;
        at = pilot_find_bytes(hs, hn, fs, fn, done);
    }
    pilot_v_buf_add(&b, hs + done, hn - done);
    pilot_val out = pilot_v_str(b.data ? b.data : "", b.len);
    free(b.data);
    return out;
}
static inline pilot_val pilot_str_byte_at(pilot_val s, pilot_val k) {
    int64_t i = pilot_v_as_i64(k);
    if (s.tag == PILOT_V_STR && i >= 0 && i < (int64_t)pilot_v_len(s)) return pilot_v_int((unsigned char)pilot_v_bytes(&s)[i]);
    char m[128];
    snprintf(m, sizeof m, "byte index %" PRId64 " out of bounds for length %" PRIu64, i, (uint64_t)(s.tag == PILOT_V_STR ? pilot_v_len(s) : 0));
    pilot_trap(m);
    return PILOT_V_NULL;
}
/* Whether `s[0..n)` is well-formed UTF-8 (RFC 3629 section 4): no overlong
   forms, no surrogates, nothing past U+10FFFF. */
static inline bool pilot_v_utf8_ok(const unsigned char *s, size_t n) {
    size_t i = 0;
    while (i < n) {
        unsigned char b = s[i];
        size_t len = b < 0x80 ? 1 : b >= 0xC2 && b <= 0xDF ? 2 : b >= 0xE0 && b <= 0xEF ? 3 : b >= 0xF0 && b <= 0xF4 ? 4 : 0;
        if (len == 0 || i + len > n) return false;
        unsigned char lo = 0x80, hi = 0xBF;
        if (b == 0xE0) lo = 0xA0;
        else if (b == 0xED) hi = 0x9F;
        else if (b == 0xF0) lo = 0x90;
        else if (b == 0xF4) hi = 0x8F;
        for (size_t k = 1; k < len; k++) {
            unsigned char c = s[i + k];
            if (c < (k == 1 ? lo : 0x80) || c > (k == 1 ? hi : 0xBF)) return false;
        }
        i += len;
    }
    return true;
}
/* `str_from_bytes`: the str whose UTF-8 bytes are the list's elements, or
   none when they are not valid UTF-8. */
static inline pilot_val pilot_str_from_bytes(pilot_val l) {
    if (l.tag != PILOT_V_LIST) pilot_v_type_error("from_utf8", l, PILOT_V_NULL);
    size_t n = (size_t)l.as.l->len;
    char *s = pilot_v_strroom(n, 0);
    for (size_t i = 0; i < n; i++) {
        pilot_val b = l.as.l->items[i];
        int64_t x = b.tag == PILOT_V_INT ? b.as.i : -1;
        if (x < 0 || x > 255) {
            pilot_v_strfree(s);
            pilot_trap("`str_from_bytes` takes bytes (0 to 255)");
        }
        s[i] = (char)x;
    }
    if (!pilot_v_utf8_ok((const unsigned char *)s, n)) {
        pilot_v_strfree(s);
        return PILOT_V_NULL;
    }
    return pilot_v_str_done(s, n);
}
static inline pilot_val pilot_rt_fault(pilot_val message) {
    pilot_trap_text(message);
    return PILOT_V_NULL;
}
static inline pilot_val pilot_seam_open(pilot_val id, bool has_note, pilot_val note) {
    pilot_val text = pilot_v_concat(pilot_v_concat(pilot_v_cstr("SEAM001: unconnected seam `"), id), pilot_v_cstr("`: "));
    if (has_note) text = pilot_v_concat(text, note);
    pilot_trap_text(text);
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
    for (int i = 1; i < pilot_argc; i++) l.as.l->items[l.as.l->len++] = pilot_v_cstr(pilot_argv[i]);
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
