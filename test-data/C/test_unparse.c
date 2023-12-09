// gcc -O0 -g -o test_unparse -I/tmp test_unparse.c /tmp/out.c

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

#include "out.h"

#define SEQ_LEN 4096

unsigned char* input = "var a=((((-9223372036854775808/-1++))));\\n";

int main (void) {
    size_t input_len = strlen(input);
    size_t* sequence = calloc(SEQ_LEN, sizeof(size_t));
    
    size_t seq_len = unparse_sequence(sequence, SEQ_LEN, input, input_len);
    assert(seq_len > 0);
    
    for (size_t i = 0; i < seq_len; ++i) {
        printf("  seq[%lu] = %lu\n", i, sequence[i]);
    }
    
    unsigned char* output = malloc(input_len);
    
    size_t out_len = serialize_sequence(sequence, seq_len, output, input_len);
    printf("input_len=%d out_len=%d\n", input_len, out_len);
    assert(out_len == input_len);
    
    output[out_len] = 0;
    printf("%s\n", output);
}
