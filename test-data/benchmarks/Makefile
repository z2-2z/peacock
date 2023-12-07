
all: baseline nop

baseline: baseline.c
	gcc -o $@ -O3 $<
	
nop: nop.c
	$(AFL_PATH)/afl-clang-lto -o $@ -O3 $<
