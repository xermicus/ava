# Code golfing: fibonacci

Our linux robot needs an x86-64 ELF binary called `fibonacci` that reads a number N from its first command line argument and prints the first N fibonacci numbers, separated by spaces. Valid range for N is `[0, 47]`.

N is at most 47.

Example: `./fibonacci 7` prints

`0 1 1 2 3 5 8`

The problem: Space for the embedded firmware is severely constrained! I need a `fibonacci` binary of at max 16kb. Optimize the code size, the smaller that binary the better.
