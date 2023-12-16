<div align="center">
    <img align="center" src="logo.png">
    <b>~~~ fuzzing with grammar-based mutations ~~~</b>
</div>

<br/>

This project is a reimplementation of [Gramatron](https://github.com/HexHive/Gramatron) that is

- __faster__: 4x higher throughput than Gramatron or LibAFL's Gramatron implementation
- __more versatile__: usable with LibAFL, libfuzzer, in a custom AFL++ mutator or standalone
- __easier to use__: no more orchestration of different scripts to get the fuzzing campaign running, everything is batteries-included
- __extendable__: at its core, peacock is a library that you can use at your leisure to customize every step of the grammar fuzzing process
- __backwards compatible__: it works with grammars that you have already written for other tools

## How it works
mutations based on automaton walks
encoded in code instead of adjacency matrix in memory
