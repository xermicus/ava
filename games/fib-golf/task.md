# Code golfing: fibonacci

Our linux robot needs an x86-64 ELF binary callded `fibonacci` that reads a number N from its first command line argument and prints the first N fibonacci numbers, separated by spaces.

Example: `./fibonacci 7` prints

`0 1 1 2 3 5 8`

The problem: Space for the embedded firmware is severely constrained! Optimize the code size, the smaller that binary the better.
