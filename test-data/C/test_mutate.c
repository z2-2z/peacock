// clang -o test_mutate -Wall -Wextra -Wpedantic -Werror -O0 -g -fsanitize=address,undefined test_mutate.c /tmp/out.c

#include <stdio.h>
#include <stdlib.h>
#include <assert.h>

size_t mutate_sequence (void* buf, size_t len, size_t capacity);
size_t serialize_sequence (size_t* seq, size_t seq_len, unsigned char* out, size_t out_len);

#define BUF_SIZE (16 * 1024 * 1024)

int main (void) {
    size_t* sequence = calloc(4096, sizeof(size_t));
    unsigned char* output = malloc(BUF_SIZE);
    
    // initial sequence
    size_t seq_len = mutate_sequence(sequence, 0, 4096);
    
    size_t out_len = serialize_sequence(sequence, seq_len, output, BUF_SIZE - 1);
    output[out_len] = 0;    
    printf("Initial: %s\n\n", output);
    
    // Mutate
    seq_len = mutate_sequence(sequence, seq_len / 2, 4096);
    out_len = serialize_sequence(sequence, seq_len, output, BUF_SIZE - 1);
    output[out_len] = 0;    
    printf("Mutation #1: %s\n\n", output);
    
    // Mutate
    seq_len = mutate_sequence(sequence, seq_len / 2, 4096);
    out_len = serialize_sequence(sequence, seq_len, output, BUF_SIZE - 1);
    output[out_len] = 0;    
    printf("Mutation #2: %s\n\n", output);
    
    // Mutate
    seq_len = mutate_sequence(sequence, seq_len / 2, 4096);
    out_len = serialize_sequence(sequence, seq_len, output, BUF_SIZE - 1);
    output[out_len] = 0;    
    printf("Mutation #3: %s\n\n", output);
    
    free(output);
    free(sequence);
}
