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
