/* F6 benchmark reference: string_build.pilot written directly in C, with
   the string kept in a buffer that doubles when full, as a C programmer
   appending in a loop would write it. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    size_t cap = 16, len = 0;
    char *s = malloc(cap);
    for (int i = 0; i < 20000; i++) {
        char part[32];
        int n = snprintf(part, sizeof part, "%d,", i);
        if (len + (size_t)n + 1 > cap) {
            while (len + (size_t)n + 1 > cap) cap *= 2;
            s = realloc(s, cap);
        }
        memcpy(s + len, part, (size_t)n + 1);
        len += (size_t)n;
    }
    printf("%zu\n%.10s\n", len, s);
    free(s);
    return 0;
}
