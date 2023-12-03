// clang -o test_generation -Wall -Wextra -Wpedantic -Werror -O0 -g -fsanitize=address,undefined test_generation.c /tmp/out.c

#include <stdio.h>
#include <stdlib.h>
#include <assert.h>

size_t mutate_sequence (void* buf, size_t len, size_t capacity);
size_t serialize_sequence (size_t* seq, size_t seq_len, unsigned char* out, size_t out_len);

#define BUF_SIZE (16 * 1024 * 1024)

int main (void) {
    size_t* sequence = calloc(4096, sizeof(size_t));
    unsigned char* output = malloc(BUF_SIZE);
    char buf[2];
    
    while (1) {
        size_t seq_len = mutate_sequence(sequence, 0, 4096);
        size_t out_len = serialize_sequence(sequence, seq_len, output, BUF_SIZE - 1);
        assert(out_len < BUF_SIZE);
        
        output[out_len] = 0;
        printf("%s\n", output);
        
        fgets(buf, 2, stdin);
    }
}
