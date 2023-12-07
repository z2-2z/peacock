#include <stdlib.h>
#include <stdio.h>

int main (void) {
#ifdef __AFL_INIT
    __AFL_INIT();
#endif
    _Exit(0);
}
