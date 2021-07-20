# What?

A silly demo to run FizzBuzz.  It invents a dumb, imperative, untyped, basic looking language and
implements the standard FizzBuzz test in it.  Then it parses and JIT compiles the source, finally
running it in place.

It's a very basic implementation, doing the absolute bare minimum to get the FizzBuzz going.  The
`print` function, which is the only library function available, is pretty hacky.  It calls libc
`puts()` for strings and has its own little method for printing integers, which in turn must be
postive and must not exceed 999. :grin:

It has a for-loop, but the iterator range must be specified as immediates.  As I said, whatever I
needed for FizzBuzz.

# Why?

I didn't have anything in particular to show you guys for the technical interview, especially
compiler-y and Rust-y and I've wanted to play around with Cranelift for a while now.

So hopefully this is interesting enough to talk about.

# How?

There are a bunch of tests in the `tests/` directory, most importantly the `fizzbuzz.test` file.

From the root directory:
```
cargo run -- ./tests/fizzbuzz.test

  1
  2
Fizz
  4
Buzz
Fizz
  7
  8
Fizz
Buzz
 11
Fizz
 13
 14
FizzBuzz
 16
 17
.
.
.
```
