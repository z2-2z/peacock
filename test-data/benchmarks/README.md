# Benchmarks

## Execution speed
- Baseline: 12k exec/s
- Peacock: ~8.6k exec/s (29% overhead)
- Original Gramatron: ~8.5k exec/s (30% overhead)
- LibAFL Gramatron: N/A because GramatronInput does not implement HasTargetBytes, so it cannot be used with ForkServerExecutor

## Raw throughput
Time to 1 GiB:
- Peacock: secs=4 nsecs=763573725 => ~205 MiB/s
- Original Gramatron: secs=17 nsecs=340090343 => ~60 MiB/s
- LibAFL Gramatron: secs=26 nsecs=26327460838 => ~40 MiB/s
