/* F6 benchmark reference: string_build.pilot written directly in C, with an
   immutable string copied on every concatenation as Pilot's is. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    char *s = calloc(1, 1);
    size_t len = 0;
    for (int i = 0; i < 20000; i++) {
        char part[32];
        int n = snprintf(part, sizeof part, "%d,", i);
        char *t = malloc(len + (size_t)n + 1);
        memcpy(t, s, len);
        memcpy(t + len, part, (size_t)n + 1);
        free(s);
        s = t;
        len += (size_t)n;
    }
    printf("%zu\n%.10s\n", len, s);
    free(s);
    return 0;
}
