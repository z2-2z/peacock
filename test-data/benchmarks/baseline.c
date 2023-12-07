#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <unistd.h>
#include <time.h>

#define TRIALS 50000

int main (void) {
    struct timespec start, end;
    
    clock_gettime(CLOCK_MONOTONIC, &start);
    
    for (int i = 0; i < TRIALS; ++i) {
        pid_t child;
        switch (child = fork()) {
            case -1: return 1;
            case 0: _Exit(0);
            default: {
                if (waitpid(child, NULL, 0) == -1) {
                    return 1;
                }
            }
        }
    }
    
    clock_gettime(CLOCK_MONOTONIC, &end);
    
    time_t diff_sec = end.tv_sec - start.tv_sec;
    
    printf("exec/s: %.02lf\n", (double)TRIALS / (double)diff_sec);
}
