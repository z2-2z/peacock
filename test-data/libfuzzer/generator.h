
#ifndef __PEACOCK_GENERATOR_H
#define __PEACOCK_GENERATOR_H

#include <stddef.h>
size_t mutate_sequence (size_t* buf, size_t len, size_t capacity);
size_t serialize_sequence (size_t* seq, size_t seq_len, unsigned char* out, size_t out_len);
size_t unparse_sequence (size_t* seq_buf, size_t seq_capacity, unsigned char* input, size_t input_len);

void seed_generator (size_t new_seed);


#endif /* __PEACOCK_GENERATOR_H */
