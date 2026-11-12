/*
 * runtime/c/pilot_rt_str.h: strings of the C runtime (included by pilot_rt.h
 * after `pilot_val`, `pilot_alloc` and `PILOT_LIVE`).
 *
 * A string value is 16 bytes (`pilot_val`):
 *
 *   byte 0       tag (PILOT_V_STR)
 *   bytes 1..3   header: (byte length << 1) | non-ASCII bit, 24 bits, little
 *                endian; PILOT_V_HDR_LONG (| non-ASCII bit) when the length
 *                is longer than PILOT_V_SHORT_MAX and kept in the buffer
 *   bytes 4..7   the first 4 bytes, zero past the end
 *   bytes 8..15  a string of up to PILOT_V_INLINE_MAX (12) bytes: bytes
 *                4..11, zero past the end. A longer one: its buffer's
 *                address, bit 63 set when the buffer is counted (made at run
 *                time) and clear for a literal, which is never counted
 *
 * The non-ASCII bit is set exactly when a byte is 0x80 or above, so two
 * equal strings have the same first 8 bytes, and comparing those 8 bytes
 * rejects almost every unequal pair. A short string needs no buffer: making,
 * copying and releasing it touches no memory beyond the value.
 *
 * A buffer starts with a header (reference count, room for appending,
 * written length) followed by the bytes and a 0 byte. A string's bytes
 * always start its buffer; strings sharing a buffer through in-place
 * appends are prefixes of the longest. Tests lower PILOT_V_SHORT_MAX to run
 * the long form on small strings.
 *
 * Search, comparison and equality read whole words, and on x86-64 and ARM64
 * whole vectors (AVX2 when the processor has it, else SSE2; NEON on ARM64),
 * never past a string's last byte: a short tail is read as a word that ends
 * at the last byte and starts in bytes already known to match.
 */

#define PILOT_V_HDR_LONG 0xFFFFFEu
#define PILOT_V_HDR_MAX_LEN 0x7FFFFEu
#ifndef PILOT_V_SHORT_MAX
#define PILOT_V_SHORT_MAX PILOT_V_HDR_MAX_LEN
#endif
#define PILOT_V_INLINE_MAX (PILOT_V_SHORT_MAX < 12 ? PILOT_V_SHORT_MAX : 12)
/* The largest header of a string held in its value. */
#define PILOT_V_INLINE_HDR ((uint32_t)(PILOT_V_INLINE_MAX * 2 + 1))
#define PILOT_V_COUNTED ((uint64_t)1 << 63)

/* A function kept out of line, so the fast paths that call it stay small
   enough to inline where they are used. */
#if defined(__GNUC__) || defined(__clang__)
#define PILOT_OUTLINE __attribute__((noinline))
#else
#define PILOT_OUTLINE
#endif

#if defined(__x86_64__) && (defined(__GNUC__) || defined(__clang__))
#define PILOT_X86 1
#include <immintrin.h>
#elif defined(__aarch64__) && (defined(__GNUC__) || defined(__clang__))
#define PILOT_NEON 1
#include <arm_neon.h>
#endif

/* The rank of each byte value in measured text, 0 the rarest: the module
   defines it from the Pilot table (stdlib/core/text/byte_rank.pilot). */
static const unsigned char pilot_byte_rank[256];

typedef struct pilot_v_strhdr {
    int64_t rc;
    uint64_t cap;
    uint64_t fill;
} pilot_v_strhdr;

static inline pilot_v_strhdr *pilot_v_strhead(const char *s) {
    return (pilot_v_strhdr *)(void *)(s - sizeof(pilot_v_strhdr));
}

/* ---- Words -------------------------------------------------------------- */

static inline uint32_t pilot_ld32(const char *p) { uint32_t w; memcpy(&w, p, 4); return w; }
static inline uint64_t pilot_ld64(const char *p) { uint64_t w; memcpy(&w, p, 8); return w; }
/* Big-endian loads: comparing the words as numbers orders their bytes. */
static inline uint32_t pilot_be32(const char *p) {
    const unsigned char *u = (const unsigned char *)p;
    return ((uint32_t)u[0] << 24) | ((uint32_t)u[1] << 16) | ((uint32_t)u[2] << 8) | u[3];
}
static inline uint64_t pilot_be64(const char *p) {
#if (defined(__GNUC__) || defined(__clang__)) && defined(__BYTE_ORDER__) && __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
    return __builtin_bswap64(pilot_ld64(p));
#else
    return ((uint64_t)pilot_be32(p) << 32) | pilot_be32(p + 4);
#endif
}
static inline int pilot_ctz32(uint32_t x) {
#if defined(__GNUC__) || defined(__clang__)
    return __builtin_ctz(x);
#else
    int n = 0;
    while (!(x & 1)) { x >>= 1; n++; }
    return n;
#endif
}
static inline int pilot_ctz64(uint64_t x) {
#if defined(__GNUC__) || defined(__clang__)
    return __builtin_ctzll(x);
#else
    int n = 0;
    while (!(x & 1)) { x >>= 1; n++; }
    return n;
#endif
}

#ifdef PILOT_X86
/* Whether the processor runs AVX2, asked once. */
static inline int pilot_has_avx2(void) {
    static int known = -1;
    if (known < 0) known = __builtin_cpu_supports("avx2") ? 1 : 0;
    return known;
}
#endif

/* ---- The value ---------------------------------------------------------- */

/* The fields are words, not byte arrays, so a value stays in two
   registers through the fast paths below instead of being written to
   memory a byte at a time and read back whole. */
#if defined(__BYTE_ORDER__) && __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
#define PILOT_LE 1
#endif
_Static_assert(sizeof(pilot_val) == 16 && offsetof(pilot_val, pre) == 4 && offsetof(pilot_val, as) == 8,
               "a string's bytes 0..11 are `pre` then the payload");
static inline uint32_t pilot_v_hdr(pilot_val v) { return v.hdr; }
static inline void pilot_v_set_hdr(pilot_val *v, uint32_t h) { v->hdr = h; }
/* The first 4 bytes as a big-endian number: comparing two orders them. */
static inline uint32_t pilot_v_prefix_be(pilot_val v) {
#ifdef PILOT_LE
    return __builtin_bswap32(v.pre);
#else
    return v.pre;
#endif
}
/* Bytes 4..11 of a string held in the value, as a big-endian number. */
static inline uint64_t pilot_v_tail_be(pilot_val v) {
#ifdef PILOT_LE
    return __builtin_bswap64(v.as.u);
#else
    return v.as.u;
#endif
}
/* The first `len` (up to 4) bytes of `s` as `pre` holds them. */
static inline uint32_t pilot_v_pre_of(const char *s, size_t len) {
    unsigned char b[4] = { 0, 0, 0, 0 };
    memcpy(b, s, len < 4 ? len : 4);
    uint32_t w;
    memcpy(&w, b, 4);
    return w;
}
static inline bool pilot_v_inline(pilot_val v) { return pilot_v_hdr(v) <= PILOT_V_INLINE_HDR; }
/* A string held in a buffer: the buffer's bytes. */
static inline const char *pilot_v_ptr(pilot_val v) { return (const char *)(uintptr_t)(v.as.u & ~PILOT_V_COUNTED); }
static inline bool pilot_v_counted(pilot_val v) { return !pilot_v_inline(v) && (v.as.u & PILOT_V_COUNTED); }
static inline size_t pilot_v_len(pilot_val v) {
    uint32_t h = pilot_v_hdr(v);
    return h < PILOT_V_HDR_LONG ? (size_t)(h >> 1) : (size_t)pilot_v_strhead(pilot_v_ptr(v))->fill;
}
/* The string's bytes: in the value itself (so `v` must outlive their use)
   or in its buffer. They are not followed by a 0 byte in general. */
static inline const char *pilot_v_bytes(const pilot_val *v) {
    return pilot_v_inline(*v) ? (const char *)&v->pre : pilot_v_ptr(*v);
}
static inline bool pilot_v_non_ascii(pilot_val v) { return pilot_v_hdr(v) & 1; }

/* Whether any of `s[0..n)` is 0x80 or above, 8 bytes at a time. */
static inline bool pilot_v_any_high(const char *s, size_t n) {
    size_t i = 0;
    uint64_t acc = 0;
    for (; i + 8 <= n; i += 8) acc |= pilot_ld64(s + i);
    for (; i < n; i++) acc |= (unsigned char)s[i];
    return (acc & 0x8080808080808080ull) != 0;
}

/* The header of a string of `len` bytes. */
static inline uint32_t pilot_v_hdr_for(size_t len, bool non_ascii) {
    return (len <= PILOT_V_SHORT_MAX ? (uint32_t)(len << 1) : PILOT_V_HDR_LONG) | (non_ascii ? 1u : 0u);
}

/* A string of `len` (at most PILOT_V_INLINE_MAX) bytes held in the value. */
static inline pilot_val pilot_v_str_inline(const char *bytes, size_t len, bool non_ascii) {
    pilot_val v = PILOT_V_NULL;
    v.tag = PILOT_V_STR;
    pilot_v_set_hdr(&v, pilot_v_hdr_for(len, non_ascii));
    /* Bytes 0..11 go to `pre` then the payload, which follow each other. */
    unsigned char b[12] = { 0 };
    memcpy(b, bytes, len);
    memcpy(&v.pre, b, 4);
    memcpy(&v.as.u, b + 4, 8);
    return v;
}

/* Sets the value of buffer string `v`: its header and first bytes. */
static inline void pilot_v_str_point(pilot_val *v, const char *s, size_t len, bool non_ascii, bool counted) {
    v->tag = PILOT_V_STR;
    pilot_v_set_hdr(v, pilot_v_hdr_for(len, non_ascii));
    v->pre = pilot_v_pre_of(s, len);
    v->as.u = (uint64_t)(uintptr_t)s | (counted ? PILOT_V_COUNTED : 0);
}

/* A counted buffer for a string of `len` bytes, uninitialised, with room
   for `room` bytes (0 for one that is not appended to, else at least
   `len`); returns where its bytes go. The caller fills them, then makes the
   value with `pilot_v_str_done`. */
static inline char *pilot_v_strroom(size_t len, size_t room) {
    pilot_v_strhdr *h = (pilot_v_strhdr *)pilot_alloc(sizeof(pilot_v_strhdr) + (room > len ? room : len) + 1);
    h->rc = 1;
    h->cap = room;
    h->fill = len;
    PILOT_LIVE(1);
    char *s = (char *)(h + 1);
    s[len] = 0;
    return s;
}
/* The value of the buffer `s` (from `pilot_v_strroom`) once its `len` bytes
   are written; a short string moves into the value and the buffer goes. */
static inline pilot_val pilot_v_str_done(char *s, size_t len) {
    bool high = pilot_v_any_high(s, len);
    pilot_val v = PILOT_V_NULL;
    if (len <= PILOT_V_INLINE_MAX && pilot_v_strhead(s)->cap == 0) {
        v = pilot_v_str_inline(s, len, high);
        free(pilot_v_strhead(s));
        PILOT_LIVE(-1);
        return v;
    }
    pilot_v_str_point(&v, s, len, high, true);
    return v;
}

/* A string of `len` bytes copied from `bytes`. */
static inline pilot_val pilot_v_str(const char *bytes, size_t len) {
    if (len <= PILOT_V_INLINE_MAX) return pilot_v_str_inline(bytes, len, pilot_v_any_high(bytes, len));
    char *s = pilot_v_strroom(len, 0);
    memcpy(s, bytes, len);
    pilot_val v = PILOT_V_NULL;
    pilot_v_str_point(&v, s, len, pilot_v_any_high(bytes, len), true);
    return v;
}
static inline pilot_val pilot_v_cstr(const char *s) { return pilot_v_str(s, strlen(s)); }

/* Frees buffer `s` from `pilot_v_strroom` that never became a value. */
static inline void pilot_v_strfree(char *s) {
    free(pilot_v_strhead(s));
    PILOT_LIVE(-1);
}

/* Stops the program with string `text` as the message. */
static inline void pilot_trap_text(pilot_val text) {
    size_t n = pilot_v_len(text);
    char *m = (char *)pilot_alloc(n + 1);
    memcpy(m, pilot_v_bytes(&text), n);
    m[n] = 0;
    pilot_trap(m);
}

/* A string literal: a static buffer whose count is 0, declared once per
   module (`PILOT_V_STATIC`, with whether a byte is 0x80 or above, which the
   compiler works out) and used as a value (`PILOT_V_STATIC_VAL`). */
#define PILOT_V_STATIC(name, lit, high) \
    static struct { pilot_v_strhdr h; char s[sizeof(lit)]; } name = { { 0, (high), sizeof(lit) - 1 }, lit }
static inline pilot_val pilot_v_static_str(const char *s, size_t len, bool high) {
    if (len <= PILOT_V_INLINE_MAX) return pilot_v_str_inline(s, len, high);
    pilot_val v = PILOT_V_NULL;
    pilot_v_str_point(&v, s, len, high, false);
    return v;
}
#define PILOT_V_STATIC_VAL(name) pilot_v_static_str((name).s, sizeof((name).s) - 1, (name).h.cap != 0)

/* ---- Equality ----------------------------------------------------------- */

#ifdef PILOT_X86
__attribute__((target("avx2"))) static bool pilot_eq_avx2(const char *a, const char *b, size_t n) {
    /* n > 68: bytes [4, n) in 128-byte steps, OR-reduced in registers. */
    size_t off = 4;
    while (off + 128 <= n) {
        __m256i x0 = _mm256_xor_si256(_mm256_loadu_si256((const __m256i *)(a + off)), _mm256_loadu_si256((const __m256i *)(b + off)));
        __m256i x1 = _mm256_xor_si256(_mm256_loadu_si256((const __m256i *)(a + off + 32)), _mm256_loadu_si256((const __m256i *)(b + off + 32)));
        __m256i x2 = _mm256_xor_si256(_mm256_loadu_si256((const __m256i *)(a + off + 64)), _mm256_loadu_si256((const __m256i *)(b + off + 64)));
        __m256i x3 = _mm256_xor_si256(_mm256_loadu_si256((const __m256i *)(a + off + 96)), _mm256_loadu_si256((const __m256i *)(b + off + 96)));
        __m256i acc = _mm256_or_si256(_mm256_or_si256(x0, x1), _mm256_or_si256(x2, x3));
        if (!_mm256_testz_si256(acc, acc)) return false;
        off += 128;
    }
    while (off + 32 <= n) {
        __m256i x = _mm256_xor_si256(_mm256_loadu_si256((const __m256i *)(a + off)), _mm256_loadu_si256((const __m256i *)(b + off)));
        if (!_mm256_testz_si256(x, x)) return false;
        off += 32;
    }
    if (off < n) {
        __m256i x = _mm256_xor_si256(_mm256_loadu_si256((const __m256i *)(a + n - 32)), _mm256_loadu_si256((const __m256i *)(b + n - 32)));
        return _mm256_testz_si256(x, x);
    }
    return true;
}
#endif

/* Whether bytes [4, n) of `a` and `b` are equal, given n > 4 and that
   bytes [0, 4) are. A read never passes byte n - 1: a tail shorter than a
   word is read as the word ending at n, which starts in bytes known equal. */
static PILOT_OUTLINE bool pilot_eq_from4(const char *a, const char *b, size_t n) {
    if (n <= 8) return pilot_ld32(a + n - 4) == pilot_ld32(b + n - 4);
    if (n <= 12) return pilot_ld64(a + n - 8) == pilot_ld64(b + n - 8);
    if (n <= 20) return ((pilot_ld64(a + 4) ^ pilot_ld64(b + 4)) | (pilot_ld64(a + n - 8) ^ pilot_ld64(b + n - 8))) == 0;
    if (n <= 36) {
        uint64_t d = (pilot_ld64(a + 4) ^ pilot_ld64(b + 4)) | (pilot_ld64(a + 12) ^ pilot_ld64(b + 12))
            | (pilot_ld64(a + n - 16) ^ pilot_ld64(b + n - 16)) | (pilot_ld64(a + n - 8) ^ pilot_ld64(b + n - 8));
        return d == 0;
    }
#if defined(PILOT_X86)
    if (n > 68 && pilot_has_avx2()) return pilot_eq_avx2(a, b, n);
    /* SSE2: 16-byte steps with an overlapped last step. */
    size_t off = 4;
    while (off + 16 <= n) {
        __m128i x = _mm_cmpeq_epi8(_mm_loadu_si128((const __m128i *)(a + off)), _mm_loadu_si128((const __m128i *)(b + off)));
        if (_mm_movemask_epi8(x) != 0xFFFF) return false;
        off += 16;
    }
    if (off < n) {
        __m128i x = _mm_cmpeq_epi8(_mm_loadu_si128((const __m128i *)(a + n - 16)), _mm_loadu_si128((const __m128i *)(b + n - 16)));
        return _mm_movemask_epi8(x) == 0xFFFF;
    }
    return true;
#elif defined(PILOT_NEON)
    size_t off = 4;
    while (off + 16 <= n) {
        uint8x16_t x = veorq_u8(vld1q_u8((const uint8_t *)a + off), vld1q_u8((const uint8_t *)b + off));
        if (vmaxvq_u8(x)) return false;
        off += 16;
    }
    if (off < n) {
        uint8x16_t x = veorq_u8(vld1q_u8((const uint8_t *)a + n - 16), vld1q_u8((const uint8_t *)b + n - 16));
        return vmaxvq_u8(x) == 0;
    }
    return true;
#else
    return memcmp(a + 4, b + 4, n - 4) == 0;
#endif
}

/* Whether strings `a` and `b`, at least one held in a buffer and with the
   same first 8 bytes, are equal. */
static PILOT_OUTLINE bool pilot_v_str_eq_buf(pilot_val a, pilot_val b) {
    uint32_t h = pilot_v_hdr(a);
    size_t n = pilot_v_len(a);
    if (h >= PILOT_V_HDR_LONG && n != pilot_v_len(b)) return false;
    const char *pa = pilot_v_ptr(a), *pb = pilot_v_ptr(b);
    if (pa == pb || n <= 4) return true;
    return pilot_eq_from4(pa, pb, n);
}

/* Whether strings `a` and `b` are equal. */
static inline bool pilot_v_str_eq(pilot_val a, pilot_val b) {
    /* Length, ASCII bit and first 4 bytes first. */
    if (a.hdr != b.hdr || a.pre != b.pre) return false;
    if (pilot_v_hdr(a) <= PILOT_V_INLINE_HDR) return a.as.u == b.as.u;
    return pilot_v_str_eq_buf(a, b);
}

/* ---- Ordering ----------------------------------------------------------- */

/* The first index in [from, n) where `a` and `b` differ, or n. */
static inline size_t pilot_mismatch(const char *a, const char *b, size_t from, size_t n) {
    size_t off = from;
#if defined(PILOT_X86)
    while (off + 16 <= n) {
        __m128i x = _mm_cmpeq_epi8(_mm_loadu_si128((const __m128i *)(a + off)), _mm_loadu_si128((const __m128i *)(b + off)));
        uint32_t eq = (uint32_t)_mm_movemask_epi8(x);
        if (eq != 0xFFFF) return off + (size_t)pilot_ctz32(~eq);
        off += 16;
    }
#elif defined(PILOT_NEON)
    while (off + 16 <= n) {
        uint8x16_t x = vceqq_u8(vld1q_u8((const uint8_t *)a + off), vld1q_u8((const uint8_t *)b + off));
        /* Each byte's equality as 4 bits of a 64-bit mask. */
        uint64_t eq = vget_lane_u64(vreinterpret_u64_u8(vshrn_n_u16(vreinterpretq_u16_u8(x), 4)), 0);
        if (eq != ~(uint64_t)0) return off + (size_t)(pilot_ctz64(~eq) >> 2);
        off += 16;
    }
#endif
    while (off + 8 <= n) {
        uint64_t d = pilot_ld64(a + off) ^ pilot_ld64(b + off);
#if defined(__BYTE_ORDER__) && __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
        if (d) return off + (size_t)(pilot_ctz64(d) >> 3);
#else
        if (d) break;
#endif
        off += 8;
    }
    while (off < n && a[off] == b[off]) off++;
    return off;
}

/* -1, 0 or 1 for strings `a` and `b` whose first 4 bytes are equal, one
   of them in a buffer, and both longer than 4 bytes. */
static PILOT_OUTLINE int pilot_v_str_cmp_long(pilot_val a, pilot_val b) {
    size_t na = pilot_v_len(a), nb = pilot_v_len(b), m = na < nb ? na : nb;
    const char *x = pilot_v_bytes(&a), *y = pilot_v_bytes(&b);
    if (x != y) {
        if (m <= 8) {
            uint32_t wa = pilot_be32(x + m - 4), wb = pilot_be32(y + m - 4);
            if (wa != wb) return wa < wb ? -1 : 1;
        } else if (m <= 12) {
            uint64_t wa = pilot_be64(x + m - 8), wb = pilot_be64(y + m - 8);
            if (wa != wb) return wa < wb ? -1 : 1;
        } else {
            size_t i = pilot_mismatch(x, y, 4, m);
            if (i < m) return (unsigned char)x[i] < (unsigned char)y[i] ? -1 : 1;
        }
    }
    return na < nb ? -1 : na > nb;
}

/* -1, 0 or 1: strings by their bytes as unsigned numbers, then by length. */
static inline int pilot_v_str_cmp(pilot_val a, pilot_val b) {
    uint32_t pa = pilot_v_prefix_be(a), pb = pilot_v_prefix_be(b);
    if (pa != pb) return pa < pb ? -1 : 1;
    uint32_t ha = pilot_v_hdr(a), hb = pilot_v_hdr(b);
    if (ha <= PILOT_V_INLINE_HDR && hb <= PILOT_V_INLINE_HDR) {
        /* Bytes 4..11 of both, zero past the end: a zero against a shorter
           string's padding leaves the length to decide. */
        uint64_t ta = pilot_v_tail_be(a), tb = pilot_v_tail_be(b);
        if (ta != tb) return ta < tb ? -1 : 1;
        uint32_t na = ha >> 1, nb = hb >> 1;
        return na < nb ? -1 : na > nb;
    }
    size_t na = pilot_v_len(a), nb = pilot_v_len(b);
    if (na <= 4 || nb <= 4) return na < nb ? -1 : na > nb;
    return pilot_v_str_cmp_long(a, b);
}

/* ---- Hashing ------------------------------------------------------------ */

static inline uint64_t pilot_mix(uint64_t a, uint64_t b) {
#if defined(__SIZEOF_INT128__)
    __uint128_t r = (__uint128_t)a * b;
    return (uint64_t)r ^ (uint64_t)(r >> 64);
#else
    uint64_t x = (a ^ (b >> 29)) * 0x9E3779B97F4A7C15ull;
    return x ^ (x >> 32);
#endif
}

/* A hash of `s[0..n)`, 8 bytes a step. */
static PILOT_OUTLINE uint64_t pilot_hash_bytes(const char *s, size_t n) {
    const uint64_t k0 = 0xA0761D6478BD642Full, k1 = 0xE7037ED1A0B428DBull;
    uint64_t h = (uint64_t)n * k1 ^ k0;
    size_t i = 0;
    for (; i + 16 <= n; i += 16) h = pilot_mix(pilot_ld64(s + i) ^ k0 ^ h, pilot_ld64(s + i + 8) ^ k1);
    if (n - i >= 8) {
        h = pilot_mix(pilot_ld64(s + i) ^ k0 ^ h, k1);
        i += 8;
    }
    uint64_t last = 0;
    for (size_t j = 0; i + j < n; j++) last |= (uint64_t)(unsigned char)s[i + j] << (8 * j);
    return pilot_mix(h ^ last, k1 ^ (uint64_t)n);
}

/* ---- Searching ---------------------------------------------------------- */

/* The positions in `n` (length m >= 2) of its two rarest bytes by
   `pilot_byte_rank`, first < second. */
static inline void pilot_rare_pair(const char *n, size_t m, size_t *first, size_t *second) {
    size_t a = 0, b = 1;
    if (pilot_byte_rank[(unsigned char)n[1]] < pilot_byte_rank[(unsigned char)n[0]]) { a = 1; b = 0; }
    for (size_t i = 2; i < m; i++) {
        unsigned char r = pilot_byte_rank[(unsigned char)n[i]];
        if (r < pilot_byte_rank[(unsigned char)n[a]]) { b = a; a = i; }
        else if (r < pilot_byte_rank[(unsigned char)n[b]]) b = i;
    }
    *first = a < b ? a : b;
    *second = a < b ? b : a;
}

/* Knuth-Morris-Pratt: the first `n` in `h[from..len)`, or -1; linear in
   all. The searcher falls back to it when anchors match often and the
   needle does not. */
static int64_t pilot_find_kmp(const char *h, size_t len, const char *n, size_t m, size_t from) {
    size_t *fail = (size_t *)pilot_alloc(sizeof(size_t) * m);
    fail[0] = 0;
    for (size_t i = 1, k = 0; i < m; i++) {
        while (k && n[i] != n[k]) k = fail[k - 1];
        if (n[i] == n[k]) k++;
        fail[i] = k;
    }
    int64_t at = -1;
    for (size_t i = from, k = 0; i < len; i++) {
        while (k && h[i] != n[k]) k = fail[k - 1];
        if (h[i] == n[k]) k++;
        if (k == m) {
            at = (int64_t)(i + 1 - m);
            break;
        }
    }
    free(fail);
    return at;
}

/* The searcher's state: the needle, its anchors, and how much verifying
   it has done against how far it has scanned. */
typedef struct pilot_finder {
    const char *h;
    size_t len;
    const char *n;
    size_t m, i1, i2;
    size_t checked;
} pilot_finder;

/* Whether `n` is at `h + at`: the anchors matched there. */
static inline bool pilot_finder_check(pilot_finder *f, size_t at) {
    f->checked += f->m;
    return memcmp(f->h + at, f->n, f->m) == 0;
}
/* Whether verifying has outrun scanning, so a linear search takes over. */
static inline bool pilot_finder_slow(const pilot_finder *f, size_t scanned) { return f->checked > 4 * scanned + 1024; }

#ifdef PILOT_X86
__attribute__((target("avx2"))) static int64_t pilot_find_avx2(pilot_finder *f, size_t pos, size_t max_pos, bool *gave_up) {
    __m256i v1 = _mm256_set1_epi8((char)f->n[f->i1]), v2 = _mm256_set1_epi8((char)f->n[f->i2]);
    size_t start = pos;
    for (; pos + 31 <= max_pos; pos += 32) {
        __m256i e1 = _mm256_cmpeq_epi8(_mm256_loadu_si256((const __m256i *)(f->h + pos + f->i1)), v1);
        __m256i e2 = _mm256_cmpeq_epi8(_mm256_loadu_si256((const __m256i *)(f->h + pos + f->i2)), v2);
        uint32_t mask = (uint32_t)_mm256_movemask_epi8(_mm256_and_si256(e1, e2));
        while (mask) {
            size_t at = pos + (size_t)pilot_ctz32(mask);
            if (pilot_finder_check(f, at)) return (int64_t)at;
            mask &= mask - 1;
        }
        if (pilot_finder_slow(f, pos - start)) { *gave_up = true; return (int64_t)pos; }
    }
    if (pos <= max_pos && max_pos >= 31) {
        /* The last 32 positions, overlapping ones already looked at. */
        size_t tail = max_pos - 31;
        __m256i e1 = _mm256_cmpeq_epi8(_mm256_loadu_si256((const __m256i *)(f->h + tail + f->i1)), v1);
        __m256i e2 = _mm256_cmpeq_epi8(_mm256_loadu_si256((const __m256i *)(f->h + tail + f->i2)), v2);
        uint32_t mask = (uint32_t)_mm256_movemask_epi8(_mm256_and_si256(e1, e2));
        mask &= ~(uint32_t)0 << (pos - tail);
        while (mask) {
            size_t at = tail + (size_t)pilot_ctz32(mask);
            if (pilot_finder_check(f, at)) return (int64_t)at;
            mask &= mask - 1;
        }
        pos = max_pos + 1;
    }
    *gave_up = false;
    return pos <= max_pos ? -2 - (int64_t)pos : -1;
}
#endif

/* Positions [pos, max_pos] whose anchor bytes both match, 16 at a time
   (SSE2 or NEON), then 8 at a time in 64-bit words (SWAR), then one at a
   time. Returns the match, -1 for none, or the position reached when
   verifying outran scanning (`gave_up`). */
static int64_t pilot_find_scan(pilot_finder *f, size_t pos, size_t max_pos, bool *gave_up) {
    const char *h = f->h;
    unsigned char b1 = (unsigned char)f->n[f->i1], b2 = (unsigned char)f->n[f->i2];
    size_t start = pos;
    *gave_up = false;
#if defined(PILOT_X86)
    __m128i v1 = _mm_set1_epi8((char)b1), v2 = _mm_set1_epi8((char)b2);
    for (; pos + 15 <= max_pos; pos += 16) {
        __m128i e1 = _mm_cmpeq_epi8(_mm_loadu_si128((const __m128i *)(h + pos + f->i1)), v1);
        __m128i e2 = _mm_cmpeq_epi8(_mm_loadu_si128((const __m128i *)(h + pos + f->i2)), v2);
        uint32_t mask = (uint32_t)_mm_movemask_epi8(_mm_and_si128(e1, e2));
        while (mask) {
            size_t at = pos + (size_t)pilot_ctz32(mask);
            if (pilot_finder_check(f, at)) return (int64_t)at;
            mask &= mask - 1;
        }
        if (pilot_finder_slow(f, pos - start)) { *gave_up = true; return (int64_t)pos; }
    }
#elif defined(PILOT_NEON)
    uint8x16_t v1 = vdupq_n_u8(b1), v2 = vdupq_n_u8(b2);
    for (; pos + 15 <= max_pos; pos += 16) {
        uint8x16_t e = vandq_u8(vceqq_u8(vld1q_u8((const uint8_t *)h + pos + f->i1), v1),
                                vceqq_u8(vld1q_u8((const uint8_t *)h + pos + f->i2), v2));
        uint64_t mask = vget_lane_u64(vreinterpret_u64_u8(vshrn_n_u16(vreinterpretq_u16_u8(e), 4)), 0) & 0x8888888888888888ull;
        while (mask) {
            size_t at = pos + (size_t)(pilot_ctz64(mask) >> 2);
            if (pilot_finder_check(f, at)) return (int64_t)at;
            mask &= mask - 1;
        }
        if (pilot_finder_slow(f, pos - start)) { *gave_up = true; return (int64_t)pos; }
    }
#endif
#if defined(__BYTE_ORDER__) && __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
    /* SWAR: a lane is 0x80 where its byte equals the splatted byte. The low
       7 bits are summed first, so a zero lane never borrows from the next. */
    const uint64_t lo7 = 0x7F7F7F7F7F7F7F7Full, hi = 0x8080808080808080ull;
    uint64_t s1 = (uint64_t)b1 * 0x0101010101010101ull, s2 = (uint64_t)b2 * 0x0101010101010101ull;
    for (; pos + 7 <= max_pos; pos += 8) {
        uint64_t z1 = pilot_ld64(h + pos + f->i1) ^ s1, z2 = pilot_ld64(h + pos + f->i2) ^ s2;
        uint64_t m1 = ~(((z1 & lo7) + lo7) | z1) & hi, m2 = ~(((z2 & lo7) + lo7) | z2) & hi;
        uint64_t mask = m1 & m2;
        while (mask) {
            size_t at = pos + (size_t)(pilot_ctz64(mask) >> 3);
            if (pilot_finder_check(f, at)) return (int64_t)at;
            mask &= mask - 1;
        }
        if (pilot_finder_slow(f, pos - start)) { *gave_up = true; return (int64_t)pos; }
    }
#endif
    for (; pos <= max_pos; pos++) {
        if ((unsigned char)h[pos + f->i1] == b1 && (unsigned char)h[pos + f->i2] == b2 && pilot_finder_check(f, pos)) return (int64_t)pos;
    }
    return -1;
}

/* The first `n` (m bytes) in `h[from..len)`, or -1. */
static int64_t pilot_find_bytes(const char *h, size_t len, const char *n, size_t m, size_t from) {
    if (m == 0) return from <= len ? (int64_t)from : -1;
    if (m > len || from > len - m) return -1;
    if (m == 1) {
        const char *hit = (const char *)memchr(h + from, n[0], len - from);
        return hit ? (int64_t)(hit - h) : -1;
    }
    pilot_finder f = { h, len, n, m, 0, 0, 0 };
    pilot_rare_pair(n, m, &f.i1, &f.i2);
    size_t max_pos = len - m, pos = from;
    bool gave_up = false;
#ifdef PILOT_X86
    if (max_pos - pos >= 31 && pilot_has_avx2()) {
        int64_t r = pilot_find_avx2(&f, pos, max_pos, &gave_up);
        if (gave_up) return pilot_find_kmp(h, len, n, m, (size_t)r);
        if (r >= -1) return r;
        pos = (size_t)(-2 - r);
    }
#endif
    int64_t r = pilot_find_scan(&f, pos, max_pos, &gave_up);
    if (gave_up) return pilot_find_kmp(h, len, n, m, (size_t)r);
    return r;
}
