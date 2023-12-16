#include <stdio.h>
#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <string.h>

#include "generator.h"

#define OUT_LEN (128 * 1024 * 1024)

size_t LLVMFuzzerCustomMutator (uint8_t* data, size_t size, size_t max_size, unsigned int seed) {
    //printf("LLVMFuzzerCustomMutator(%p, %lu, %lu, %u)\n", data, size, max_size, seed);
    
    if ((size % sizeof(size_t)) != 0) {
        size = 0;
    }
    
    size /= sizeof(size_t);
    
    if (size) {
        size = rand() % size;
    }
    
    max_size /= sizeof(size_t);
    
    seed_generator(seed);
    
    size_t new_len = mutate_sequence(
        (size_t*) data,
        size,
        max_size
    );
    
    return new_len * sizeof(size_t);
}

int LLVMFuzzerTestOneInput (const uint8_t* data, size_t size) {
    static unsigned char* output = NULL;
    
    //printf("LLVMFuzzerTestOneInput(%p, %lu)\n", data, size);
    
    if ((size % sizeof(size_t)) != 0) {
        return -1;
    }
    
    if (!output) {
        output = malloc(OUT_LEN + 1);
    }
    
    size /= sizeof(size_t);
    
    size_t new_len = serialize_sequence(
        (size_t*) data,
        size,
        output,
        OUT_LEN
    );
    output[new_len] = 0;
    
    //printf("%s\n", output);
    
    return 0;
}

void print_file (char* filename) {
    FILE* file = fopen(filename, "rb");
    fseek(file, 0, SEEK_END);
    size_t file_size = ftell(file);
    
    if ((file_size % sizeof(size_t)) != 0) {
        exit(1);
    }
    
    fseek(file, 0, SEEK_SET);
    size_t* buffer = malloc(file_size);
    fread(buffer, 1, file_size, file);
    
    unsigned char* output = malloc(OUT_LEN + 1);
    
    size_t out_len = serialize_sequence(
        buffer,
        file_size / sizeof(size_t),
        output,
        OUT_LEN
    );
    output[out_len] = 0;
    
    printf("%s\n", output);
    
    fclose(file);
}

int LLVMFuzzerInitialize (int* argcp, char*** argvp) {
    int argc = *argcp;
    char** argv = *argvp;
    
    if (argc == 2 && !strncmp(argv[1], "--print=", 8)) {
        print_file(argv[1] + 8);
        exit(0);
    }
    
    return 0;
}
