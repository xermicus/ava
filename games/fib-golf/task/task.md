# Code golfing: fibonacci

Our linux robot needs an x86-64 ELF binary called `fibonacci` that reads a number N from its first command line argument and prints the first N fibonacci numbers, separated by spaces. Valid range for N is `[0, 47]`.

N is at most 47.

Example: `./fibonacci 7` prints

`0 1 1 2 3 5 8`

The problem: Space for the embedded firmware is severely constrained! I need a `fibonacci` binary of at max 16kb. Optimize the code size, the smaller that binary the better.

The sandbox may run on another architecture, see `uname -m`. `x86_64-linux-gnu-gcc`, `x86_64-linux-gnu-g++` and `nasm` build x86-64 binaries there, `qemu-x86_64 -L /usr/x86_64-linux-gnu ./binary` runs one, and the CI runs them the same way.
