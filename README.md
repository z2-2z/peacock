<div align="center">
    <img align="center" src="logo.png">
    <b>~~~ fuzzing with grammar-based mutations ~~~</b>
</div>

<br/>

This project is a reimplementation of [Gramatron](https://github.com/HexHive/Gramatron) that is

- __performant__: 4x higher throughput than Gramatron or LibAFL's Gramatron implementation
- __versatile__: usable with LibAFL, libfuzzer, in a custom AFL++ mutator or standalone
- __easy to use__: no more orchestration of different scripts to get the fuzzing campaign running, everything is batteries-included
- __extendable__: at its core, peacock is a library that you can use at your leisure to customize every step of the grammar fuzzing process
- __backwards compatible__: it works with grammars that you have already written for other tools

## How it works
Peacock implements so-called "grammar-based mutations". This means that it will mutate its inputs in such a way that they will always adhere to a given grammar.     

The way mutations work is the same as in Gramatron. A grammar is converted to a [PDA](https://en.wikipedia.org/wiki/Pushdown_automaton) such that an input can be represented as a walk through the automaton. Then, a mutation of an input is simply a modification of an automaton walk. We cut off the walk at a random point and let it find a new random path through the automaton from there.

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
2. `peacock-dump`: peacock-fuzz saves crashes and queue items in a raw, binary format to disk. Use this tool to get a human readable output from any such file. All these binary files have the prefix `peacock-raw-`
3. `peacock-compile`: Takes a grammar and compiles it to C code
4. `peacock-merge`: Merge multiple grammar files into one or convert a grammar file from one format into another

If you want more fine-grained control you can use the crate `peacock_fuzz`, which is the backbone of all the tools from above.

Execute
```
cargo doc --open
```
in order to get started with peacock as a library.


## How to write grammars

Peacock accepts its context-free grammars in JSON format.
A context-free grammar has production rules of the form:
```
A -> X Y Z ...
```
where `A` _must_ be a non-terminal and `X`,`Y`,`Z` can be non-terminals or terminals. The right-hand-side must contain at least one symbol.

Non-terminals are enclosed in `<>`, so the non-terminal `A` would be represented as `<A>`. Terminals are enclosed in `''`.

The set of rules 
```
A -> a B
A -> a
B -> b B
B -> Ɛ
```
would be written as
```jsonc
{
    // Comments are also possible :)
    "<A>": [
        ["'a'", "<B>"],
        ["'a'"]
    ],
    "<B>": [
        ["'b'", "<B>"],
        ["''"] // Ɛ = ''
    ]
}
```
and corresponds to the regular expression `a(b*)`.

Peacock also supports the Gramatron format, which is a bit different and does not allow for comments.

## C API Documentation

- `void seed_generator (size_t new_seed)`   
  Supply a seed for the RNG of the mutator.
- `size_t unparse_sequence (size_t* seq_buf, size_t seq_capacity, unsigned char* input, size_t input_len)`   
  Given an input that adheres to the grammar, find the corresponding automaton walk. _This function may be slow, use outside of hot loop._
  - `seq_buf`: Automaton walk will be written into this buffer
  - `seq_capacity`: Maximum number of elements that `seq_buf` can hold (not number of bytes)
  - `input`: User input adhering to grammar
  - `input_len`: Length of `input`
  
  Returns the number of elements written to `seq_buf` or 0 if input does not adhere to grammar.
- `size_t mutate_sequence (size_t* buf, size_t len, size_t capacity)`   
  Given an automaton walk, create a random mutant of the walk.
  - `buf`: Pointer to array that holds automaton walk
  - `len`: Number of items in `buf` (not number of bytes)
  - `capacity`: Maximum number of items that `buf` can hold (not number of bytes)
  
  Returns the length of the new walk.
- `size_t serialize_sequence (size_t* seq, size_t seq_len, unsigned char* out, size_t out_len)`    
  Given an automaton walk, create the corresponding output.
  - `seq`: Pointer to automaton walk
  - `seq_len`: Number of items in `seq` (not number of bytes)
  - `out`: Output will be written into that buffer
  - `out_len`: Number of bytes in `out`
  
  Returns how many bytes have been written to `out`.
  
  
Macros:
- `MULTITHREADING`: Define this variable to make the mutator completely thread-safe
- `MAKE_VISIBLE`: Set visibility of functions from above to default
- `SEED`: Compile-time seed for the RNG
- `DISABLE_rand`: Don't include the internal rand function and call an external one with the signature `size_t rand (void)`
- `DISSABLE_seed`: Don't include the `seed_generator` function and call an external one with the same signature from above.

## Warning
This project is currently in a beta stage. Not all features are implemented yet and bugs will occur.
