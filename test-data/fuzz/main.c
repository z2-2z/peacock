// 

#include <stdio.h>
#include <stdlib.h>

void print_loop (FILE* input) {
    char buf[128];
    
    while (!feof(input) && !ferror(input)) {
        size_t num = fread(buf, 1, sizeof(buf), input);
        
        if (!num) {
            break;
        }
        
        fwrite(buf, 1, num, stdout);
    }
    
    fprintf(stdout, "\n");
    fflush(stdout);
}

int main (int argc, char** argv) {
    __AFL_INIT();
    
    FILE* input = NULL;
    
    if (argc == 1) {
        input = stdin;
    } else if (argc == 2) {
        input = fopen(argv[1], "rb");
    } else {
        fprintf(stderr, "Invalid test invocation\n");
        return 1;
    }
    
    print_loop(input);
    
    return 0;
}
