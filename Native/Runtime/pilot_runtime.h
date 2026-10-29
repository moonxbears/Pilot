#ifndef PILOT_RUNTIME_H
#define PILOT_RUNTIME_H

/**
 * Pilot Zero-Dependency Native Runtime Header
 * Provides lightweight memory allocation, string utilities, basic collections,
 * assertion checking, and C-ABI export bindings for Native Ahead-Of-Time (AOT) execution.
 */

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32) || defined(__CYGWIN__)
  #define PILOT_API __declspec(dllexport)
#elif defined(__GNUC__) || defined(__clang__)
  #define PILOT_API __attribute__((visibility("default")))
#else
  #define PILOT_API
#endif

/* --- Basic Types --- */
typedef int64_t  pilot_int_t;
typedef double   pilot_float_t;
typedef bool     pilot_bool_t;

typedef struct pilot_str {
    const char* data;
    size_t length;
} pilot_str_t;

/* --- Milestone 15.3: Zero-Marshalling C-ABI Loop & Slice Types --- */
typedef struct pilot_range_window_i32 {
    int32_t current; /* Offset: 0 */
    int32_t max;     /* Offset: 4 (sizeof == 8, 0 padding) */
} pilot_range_window_i32_t;

typedef struct pilot_range_window_i64 {
    int64_t current; /* Offset: 0 */
    int64_t max;     /* Offset: 8 (sizeof == 16, 0 padding) */
} pilot_range_window_i64_t;

typedef struct pilot_slice_buffer {
    void*  data_ptr; /* Offset: 0 */
    size_t length;   /* Offset: 8 */
    size_t capacity; /* Offset: 16 (sizeof == 24 on 64-bit targets) */
} pilot_slice_buffer_t;

typedef struct pilot_slice_buffer_f64 {
    double* data_ptr;
    size_t  length;
    size_t  capacity;
} pilot_slice_buffer_f64_t;

typedef struct pilot_slice_buffer_i64 {
    int64_t* data_ptr;
    size_t   length;
    size_t   capacity;
} pilot_slice_buffer_i64_t;

typedef struct pilot_slice_buffer_i32 {
    int32_t* data_ptr;
    size_t   length;
    size_t   capacity;
} pilot_slice_buffer_i32_t;

/* --- Memory Management --- */
static inline void* pilot_alloc(size_t size) {
    void* ptr = malloc(size);
    if (!ptr && size > 0) {
        fprintf(stderr, "Pilot Native Runtime: Out of memory (allocation of %zu bytes failed)\n", size);
        exit(137);
    }
    return ptr;
}

static inline void pilot_free(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

/* --- String Utilities --- */
static inline pilot_str_t pilot_str_create(const char* s) {
    pilot_str_t str;
    str.data = s ? s : "";
    str.length = s ? strlen(s) : 0;
    return str;
}

static inline void pilot_print_str(pilot_str_t s) {
    if (s.data && s.length > 0) {
        fwrite(s.data, 1, s.length, stdout);
    }
}

static inline void pilot_println_str(pilot_str_t s) {
    pilot_print_str(s);
    fputc('\n', stdout);
}

static inline void pilot_print_int(pilot_int_t val) {
    printf("%lld", (long long)val);
}

static inline void pilot_println_int(pilot_int_t val) {
    printf("%lld\n", (long long)val);
}

static inline void pilot_print_float(pilot_float_t val) {
    printf("%g", val);
}

static inline void pilot_println_float(pilot_float_t val) {
    printf("%g\n", val);
}

static inline void pilot_print_bool(pilot_bool_t val) {
    fputs(val ? "true" : "false", stdout);
}

static inline void pilot_println_bool(pilot_bool_t val) {
    fputs(val ? "true\n" : "false\n", stdout);
}

/* --- Assertions & Panics --- */
static inline void pilot_panic(const char* message) {
    fprintf(stderr, "PILOT RUNTIME PANIC: %s\n", message ? message : "Unknown error");
    exit(1);
}

static inline void pilot_assert(bool condition, const char* message) {
    if (!condition) {
        pilot_panic(message);
    }
}

/* --- Dynamic Array / Buffer --- */
typedef struct pilot_list {
    pilot_int_t* items;
    size_t count;
    size_t capacity;
} pilot_list_t;

static inline pilot_list_t* pilot_list_create(size_t initial_capacity) {
    pilot_list_t* list = (pilot_list_t*)pilot_alloc(sizeof(pilot_list_t));
    list->capacity = initial_capacity < 4 ? 4 : initial_capacity;
    list->count = 0;
    list->items = (pilot_int_t*)pilot_alloc(list->capacity * sizeof(pilot_int_t));
    return list;
}

static inline void pilot_list_append(pilot_list_t* list, pilot_int_t item) {
    if (!list) return;
    if (list->count >= list->capacity) {
        list->capacity *= 2;
        list->items = (pilot_int_t*)realloc(list->items, list->capacity * sizeof(pilot_int_t));
        if (!list->items) {
            pilot_panic("Out of memory resizing list");
        }
    }
    list->items[list->count++] = item;
}

static inline pilot_int_t pilot_list_get(const pilot_list_t* list, size_t index) {
    if (!list || index >= list->count) {
        pilot_panic("List index out of bounds");
    }
    return list->items[index];
}

static inline void pilot_list_destroy(pilot_list_t* list) {
    if (list) {
        pilot_free(list->items);
        pilot_free(list);
    }
}

/* --- Milestone 21+: C++20 `libpilot_rt` Shared Accelerator C-ABI --- */
PILOT_API int32_t pilot_rt_version(void);
PILOT_API int32_t pilot_rt_simd_width_bits(void);

PILOT_API double  pilot_rt_slice_sum_f64(const double* data, size_t len);
PILOT_API int64_t pilot_rt_slice_sum_i64(const int64_t* data, size_t len);
PILOT_API double  pilot_rt_slice_dot_f64(const double* a, const double* b, size_t len);
PILOT_API void    pilot_rt_slice_saxpy_f64(double alpha, const double* x, double* y, size_t len);
PILOT_API void    pilot_rt_slice_scale_f64(double alpha, double* data, size_t len);
PILOT_API void    pilot_rt_slice_fill_f64(double* data, size_t len, double value);
PILOT_API void    pilot_rt_slice_fill_i64(int64_t* data, size_t len, int64_t value);
PILOT_API void    pilot_rt_slice_sort_f64(double* data, size_t len);
PILOT_API void    pilot_rt_slice_sort_i64(int64_t* data, size_t len);

PILOT_API void    pilot_rt_welford_f64(
    const double* data,
    size_t len,
    double* out_mean,
    double* out_m2,
    double* out_min,
    double* out_max);

PILOT_API void    pilot_rt_integrate_vectors_f64(
    double* positions,
    double* velocities,
    double dt,
    double drag,
    size_t count);

/* Returns: 1 = matched (*out_start, *out_end), 0 = no match, -1 = fallback to Unicode predicate matcher */
PILOT_API int32_t pilot_rt_dfa_find_utf16(
    const int32_t* ascii_table_128,
    const uint8_t* accept_flags,
    int32_t state_count,
    int32_t has_non_ascii,
    int32_t anchored_start,
    int32_t anchored_end,
    const uint16_t* text,
    int32_t text_len,
    int32_t start_at,
    int32_t* out_start,
    int32_t* out_end);

PILOT_API int32_t pilot_rt_dfa_find_utf8(
    const int32_t* ascii_table_128,
    const uint8_t* accept_flags,
    int32_t state_count,
    int32_t has_non_ascii,
    int32_t anchored_start,
    int32_t anchored_end,
    const uint8_t* bytes,
    int32_t byte_len,
    int32_t start_at,
    int32_t* out_start,
    int32_t* out_end);

#ifdef __cplusplus
}
#endif

#endif /* PILOT_RUNTIME_H */
