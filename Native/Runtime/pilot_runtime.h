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

/* --- Basic Types --- */
typedef int64_t  pilot_int_t;
typedef double   pilot_float_t;
typedef bool     pilot_bool_t;

typedef struct pilot_str {
    const char* data;
    size_t length;
} pilot_str_t;

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

#ifdef __cplusplus
}
#endif

#endif /* PILOT_RUNTIME_H */
