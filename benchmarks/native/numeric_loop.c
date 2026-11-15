/* F6 benchmark reference: numeric_loop.pilot written directly in C. */
#include <inttypes.h>
#include <stdio.h>

int main(void) {
    int64_t total = 0;
    for (int64_t i = 0; i < 49000000; i++) total += (i * i) % 7;
    printf("%" PRId64 "\n", total);
    return 0;
}
