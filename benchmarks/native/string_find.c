/* Native benchmark reference: string_find.pilot in C, searching with
   strstr as a C programmer would. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char *repeat(const char *piece, size_t n, const char *tail) {
    size_t p = strlen(piece), t = strlen(tail);
    char *s = malloc(p * n + t + 1);
    for (size_t i = 0; i < n; i++) memcpy(s + i * p, piece, p);
    memcpy(s + p * n, tail, t + 1);
    return s;
}

int main(void) {
    char *text = repeat("the quick brown fox jumps over the lazy dog ", 20000, "expression");
    long at = -1;
    for (int k = 0; k < 20; k++) {
        char *hit = strstr(text, "expression");
        at = hit ? (long)(hit - text) : -1;
    }
    printf("%ld\n", at);
    long count = 0;
    for (char *p = text; (p = strstr(p, "lazy")) != NULL; p++) count++;
    printf("%ld\n", count);
    char *flat = repeat("a", 100000, "b");
    char *needle = repeat("a", 20, "b");
    long found = -1;
    for (int k = 0; k < 20; k++) {
        char *hit = strstr(flat, needle);
        found = hit ? (long)(hit - flat) : -1;
    }
    printf("%ld\n", found);
    free(text);
    free(flat);
    free(needle);
    return 0;
}
