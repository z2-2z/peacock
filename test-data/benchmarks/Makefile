
all: baseline nop throughput

baseline: baseline.c
	gcc -o $@ -O3 $<
	
nop: nop.c
	$(AFL_PATH)/afl-clang-lto -o $@ -O3 $<
	
throughput: throughput.c output/generator.c
	clang -Ofast -march=native -fomit-frame-pointer -fno-stack-protector -o $@ $^ -Ioutput
