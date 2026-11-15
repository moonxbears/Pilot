/* Native benchmark reference: string_compare.pilot in C, with the strings
   as NUL-terminated buffers compared by strcmp, as a C programmer would. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    static char short_s[1000][8], long_s[1000][40];
    for (int i = 0; i < 1000; i++) {
        snprintf(short_s[i], sizeof short_s[i], "id%02d", i % 100);
        snprintf(long_s[i], sizeof long_s[i], "pilot/module/path/component_id%02d", i % 100);
    }
    long eq_s = 0, lt_s = 0, eq_l = 0, lt_l = 0;
    for (int r = 0; r < 50; r++) {
        for (int i = 0; i < 1000; i++) {
            for (int d = 1; d <= 100; d++) {
                int j = (i + d) % 1000;
                int cs = strcmp(short_s[i], short_s[j]);
                int cl = strcmp(long_s[i], long_s[j]);
                if (cs == 0) eq_s++;
                if (cs < 0) lt_s++;
                if (cl == 0) eq_l++;
                if (cl < 0) lt_l++;
            }
        }
    }
    printf("%ld\n%ld\n%ld\n%ld\n", eq_s, lt_s, eq_l, lt_l);
    return 0;
}
