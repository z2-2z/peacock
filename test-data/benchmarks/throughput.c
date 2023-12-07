#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "generator.h"

#define SEQ_LEN  4096
#define BUF_SIZE (128 * 1024 * 1024)

int main (void) {
    size_t* sequence = calloc(SEQ_LEN, sizeof(size_t));
    unsigned char* output = malloc(BUF_SIZE);
    struct timespec start, end;
    size_t generated = 0;
    
    clock_gettime(CLOCK_MONOTONIC, &start);
    
    while (generated < 1 * 1024 * 1024 * 1024) {
        size_t seq_len = mutate_sequence(sequence, 0, SEQ_LEN);
        size_t out_len = serialize_sequence(sequence, seq_len, output, BUF_SIZE);
        generated += out_len;
    }
    
    clock_gettime(CLOCK_MONOTONIC, &end);
    
    time_t secs = end.tv_sec - start.tv_sec;
    long  nsecs = end.tv_nsec - start.tv_nsec;
    
    if (nsecs < 0) {
        secs -= 1;
        nsecs += 1000000000;
    }
    
    printf("secs=%lu nsecs=%ld\n", secs, nsecs);
}
