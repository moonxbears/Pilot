#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
struct Trio { int32_t a, b, c; };
struct ByteThenTrio { uint8_t x; struct Trio t; };
struct ByteThenWide { uint8_t a; __int128 b; };
struct Flags { bool a; bool b; int32_t c; };
struct Mixed { uint8_t a; double b; uint16_t c; };          /* declaration order */
struct MixedSorted { double b; uint16_t c; uint8_t a; };    /* by descending alignment */
struct Nested { struct ByteThenTrio n; uint8_t z; };
int main(void) {
    printf("%zu %zu %zu %zu %zu %zu %zu %zu\n", sizeof(struct Trio), sizeof(struct ByteThenTrio),
        sizeof(struct ByteThenWide), sizeof(struct Flags), sizeof(struct Mixed), sizeof(struct MixedSorted),
        sizeof(struct Nested), _Alignof(struct ByteThenWide));
    return 0;
}
