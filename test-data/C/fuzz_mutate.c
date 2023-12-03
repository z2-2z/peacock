// clang -o fuzz_mutate -Wall -Wextra -Wpedantic -Werror -O0 -g -fsanitize=address,undefined fuzz_mutate.c /tmp/out.c

#include <stdio.h>
#include <stdlib.h>

size_t mutate_sequence (void* buf, size_t len, size_t capacity);

int main (void) {
    while (1) {
        size_t capacity = rand() % 256;
        void* buf = calloc(capacity, sizeof(size_t));
        
        size_t len = 0;
        
        if (capacity > 0 && (rand() % 4) == 3) {
            len = rand() % (capacity + 1);
        }
        
        size_t new_len = mutate_sequence(buf, len, capacity);
        
        printf("capacity=%lu old_len=%lu new_len=%lu\n", capacity, len, new_len);
        
        free(buf);
    }
}
