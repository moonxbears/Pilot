/* F6 benchmark reference: records.pilot written directly in C, with the
   bodies in an array of structs updated in place. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>

typedef struct { int64_t pos, vel; } Body;
static Body bodies[1000];

int main(void) {
    for (int64_t i = 0; i < 1000; i++) bodies[i] = (Body){ i * 10, i % 7 - 3 };
    for (int step = 0; step < 2000; step++) {
        for (int j = 0; j < 1000; j++) {
            int64_t p = bodies[j].pos + bodies[j].vel;
            int64_t v = (p < 0 || p > 10000) ? -bodies[j].vel : bodies[j].vel;
            bodies[j] = (Body){ p, v };
        }
    }
    int64_t pos = 0, vel = 0;
    for (int i = 0; i < 1000; i++) { pos += bodies[i].pos; vel += bodies[i].vel; }
    printf("%" PRId64 "\n%" PRId64 "\n", pos, vel);
    return 0;
}
