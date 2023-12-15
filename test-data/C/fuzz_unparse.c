// clang -o fuzz_unparse -Wall -Wextra -Wpedantic -Werror -O0 -g -fsanitize=address,undefined -I/tmp fuzz_unparse.c /tmp/out.c

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <time.h>

#include "out.h"

#define SEQ_LEN 4096
#define BUF_LEN (128 * 1024 * 1024)

int main (void) {
    seed_generator(time(NULL));
    size_t* generated = calloc(SEQ_LEN, sizeof(size_t));
    size_t* unparsed = calloc(SEQ_LEN, sizeof(size_t));
    unsigned char* output = malloc(BUF_LEN + 1);
    unsigned char* output2 = malloc(BUF_LEN + 1);
    size_t i = 0;
    
    while (1) {
        printf("Iter %lu\n", i++);
        
        size_t gen_len = mutate_sequence(generated, 0, SEQ_LEN);
        size_t out_len = serialize_sequence(generated, gen_len, output, BUF_LEN);
        size_t unp_len = unparse_sequence(unparsed, SEQ_LEN, output, out_len);
        size_t out2_len = serialize_sequence(unparsed, unp_len, output2, BUF_LEN);
        
        output[out_len] = 0;
        output2[out2_len] = 0;
        
        if (out_len != out2_len || strcmp((const char*) output, (const char*) output2)) {
            printf("out_len = %lu\n", out_len);
            printf("out2_len = %lu\n", out2_len);
            
            printf("--- GENERATED ---\n");
            printf("%s\n", output);
            
            printf("--- UNPARSED ---\n");
            printf("%s\n", output2);
            
            break;
        }
    }
}
