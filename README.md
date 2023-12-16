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
Peacock implements so-called "grammar-based mutations". This means that it will mutate its inputs in such a way that they will always adhere to a given grammar.     

The way mutations work is the same as in Gramatron. A grammar is converted into a [PDA](https://en.wikipedia.org/wiki/Pushdown_automaton) such that an input can be represented as a walk through the automaton. Then, a mutation of an input is simply a modification of an automaton walk. We cut off the walk at a random point and let it find a new random path through the automaton from there.

While Gramatron and LibAFL realize the automaton as an adjacency matrix,
peacock generates C code that encodes the automaton in its control flow. This saves us a lot of memory accesses and makes the mutation procedure faster.

The generated C code exposes a certain API that can be used by any application, e.g. a libfuzzer harness, an AFL++ custom mutator or even Rust code.

But peacock also ships a ready to use fuzzer that can fuzz any binary that has been compiled with AFL++'s compilers or implements an AFL-style forkserver.

## How to use it
Clone the repo and execute
```
cargo build --release
```
This creates 4 ready-to-use tools:

1. `peacock-fuzz`: A coverage-guided fuzzer that can fuzz any binary compiled with AFL++'s compilers or anything that speaks AFL's forkserver protocol
2. `peacock-dump`: peacock-fuzz saves crashes and queue items in a raw, binary format to disk. Use this tool to get a human readable output from any such file. All these binary files have the prefix `peacock-raw-`.
3. `peacock-gen`: Takes a grammar and produces C code
4. `peacock-merge`: Merge multiple grammar files into one or convert a grammar file from one format into another.

If you want more fine-grained control you can use the crate `peacock_fuzz` that is the backbone of all of the tools above.
Execute
```
cargo doc --open
```
in order to get started with peacock as a library.


## How to write Grammars

## C API Documentation
