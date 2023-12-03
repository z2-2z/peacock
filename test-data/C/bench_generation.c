// gcc -o bench_generation -Wall -Wextra -Wpedantic -Werror -O3 bench_generation.c /tmp/out.c

#include <stdio.h>
#include <stdlib.h>
#include <time.h>

size_t mutate_sequence (void* buf, size_t len, size_t capacity);
size_t serialize_sequence (size_t* seq, size_t seq_len, unsigned char* out, size_t out_len);

#define BUF_SIZE (1 * 1024 * 1024)

int main (void) {
    size_t* sequence = calloc(4096, sizeof(size_t));
    unsigned char* output = malloc(BUF_SIZE);
    size_t generated = 0;
    struct timespec start;
    struct timespec now;
    size_t trials = 0;
    
    clock_gettime(CLOCK_MONOTONIC, &start);
    
    while (1) {
        size_t seq_len = mutate_sequence(sequence, 0, 4096);
        size_t out_len = serialize_sequence(sequence, seq_len, output, BUF_SIZE);
        generated += out_len;
        trials++;
        
        if ((generated % 1048576) == 0) {
            clock_gettime(CLOCK_MONOTONIC, &now);
            
            double secs = (double) (now.tv_sec - start.tv_sec);
            double amount = (double) (generated / 1048576);
            printf("Generated >= %.02lf MiB/s | Avg. size: %lu\n", amount / secs, generated / trials);
        }
    }
}
