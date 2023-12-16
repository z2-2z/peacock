
harness: harness.c generator.c
	clang -O3 -flto -fsanitize=fuzzer -o $@ -I. $^
