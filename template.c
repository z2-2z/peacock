#error "This is just for brainstorming"

typedef struct {
    size_t* buf;
    size_t len;
    size_t capacity;
} Sequence;

static int generate_seq_SNGLE (Sequence* seq, size_t* step) {
    size_t idx = seq->len;
    
    if (*step >= idx) {
        if (idx >= seq->capacity) {
            return 0;
        }
        
        seq->buf[idx] = 0;
        seq->len = idx + 1;
    }
    
     *step += 1;
    
    // code inside case
    
    return 1;
}

static int generate_seq_ENTRYPOINT (Sequence* seq, size_t* step) {
    size_t idx = seq->len;
    size_t target;
    
    if (*step < idx) {
        target = seq->buf[*step];
    } else {
        if (idx >= seq->capacity) {
            return 0;
        }
        
        target = rand() % 2;
        seq->buf[idx] = target;
        seq->len = idx + 1;
    }
    
    *step += 1;
    
    switch (target) {
        case 0: {
            if (!generate_seq_A(seq, step)) {
                return 0;
            }
            
            // repeat for all other non-terminals in rule
            
            break;
        }
        
        case 1: {
            // no non-terminals to explore
            
            break;
        }
        
        default: {
            __builtin_unreachable();
        },
    }
    
    return 1;
}

// In rust: Vec<usize>
size_t generate_sequence (void* buf, size_t len, size_t capacity) {
    if (UNLIKELY(!buf || !capacity)) {
        return 0;
    }
    
    Sequence seq = {
        .buf = (size_t*) buf,
        .len = len,
        .capacity = capacity,
    };
    size_t step = 0;
    
    generate_seq_ENTRYPOINT(&seq, &step);
    
    return seq.len;
}

static const unsigned char term0[] = {...};

static size_t serialize_seq_ENTRYPOINT (size_t* seq, size_t seq_len, unsigned char* out, size_t out_len, size_t* step) {
    if (UNLIKELY(*step >= seq_len)) {
        return 0;
    }
    
    unsigned char* original_out = out;
    size_t target = seq[*step];
    *step += 1;
    
    switch (target) {
        case 0: {
            // non-terminal
            size_t len = serialize_seq_NONTERM(seq, seq_len, out, out_len, step);
            out += len; out_len -= len;
            
            // terminal
            if (UNLIKELY(out_len < sizeof(term0))) {
                goto end;
            }
            __builtin_memcpy_inline(out, term0, sizeof(term0));
            out += sizeof(term0); out_len -= sizeof(term0);
            //TODO: optimize for 1, 2, 4, 8
            
            break;
        }
        
        default: {
            __builtin_unreachable();
        }
    }
    
  end:
    return (size_t) (out - original_out);
}

size_t serialize_sequence (size_t* seq, size_t seq_len, unsigned char* out, size_t out_len) {
    if (UNLIKELY(!seq || !seq_len || !out || !out_len)) {
        return 0;
    }
    
    size_t step = 0;
    
    return serialize_seq_ENTRYPOINT(seq, seq_len, out, out_len, &step);
}


static int unparse_sequence_nontermXYZ (Sequence* seq, unsigned char* input, size_t input_len, size_t* cursor) {
    size_t seq_idx = seq->len;
    
    if (UNLIKELY(seq_idx >= seq->capacity)) {
        return 0;
    }
    
    // Single rule
    seq->len = seq_idx + 1;
    do {
        size_t tmp_cursor = *cursor;
        
        // try item 1: terminal
        if (UNLIKELY(input_len - tmp_cursor < sizeof(TERMX)) || __builtin_memcmp(&input[tmp_cursor], TERMX, sizeof(TERMX)) != 0) {
            break;
        }
        tmp_cursor += sizeof(TERMX);
        
        // try item 2: non-terminal
        if (!unparse_sequence_nontermABC(seq, input, input_len, &tmp_cursor)) {
            break;
        }
        
        *cursor = tmp_cursor;
        seq->buf[seq_idx] = 0; // index of rule
        return 1;
    } while(0);
    
    seq->len = seq_idx;
    return 0;
}

size_t unparse_sequence (size_t* seq_buf, size_t seq_capacity, unsigned char* input, size_t input_len) {
    if (UNLIKELY(!seq_buf || !seq_capacity || !input || !input_len)) {
        return 0;
    }
    
    Sequence seq = {
        .buf = seq_buf,
        .len = 0,
        .capacity = seq_capacity,
    };
    size_t cursor = 0;
    unparse_sequence_nontermXYZ(&seq, input, input_len, &cursor);
    return seq.len;
}
