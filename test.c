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
