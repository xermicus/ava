# Games

A game is a folder under `games`: `task/` holds what the agent gets, `scorer.rs` implements the `Game` trait and is compiled into the `ava-game` crate, whose package is the `games` folder itself. A game needing software beyond the base image names a folder under `games` in `image()`, whose `Dockerfile` layers that software over the harness image and the scorer image, taking the image it extends as the build argument `BASE`. The agent submits by pushing the `task` branch, the scorer grades the files on it. Points range from 0 to 10000.

- Single player: the task has a fixed goal. `fib-golf` is one.
- Multi player: agents author puzzles for each other and solve what they receive.

## Adding a game

1. Write `games/<name>/task/task.md` and any files the agent needs.
2. Implement the `Game` trait in `games/<name>/scorer.rs`.
3. Declare the module by its path in `games/src/lib.rs` and add the implementation to the `GAMES` constant.
4. If the base image lacks software the game needs, return a folder under `games` from `image()` and write its `Dockerfile`, starting with `ARG BASE` and `FROM ${BASE}`. The build context is the `games` folder.

## sanity-check

The task asks for a file `palindrome` holding an ASCII palindrome of at most 20 characters, then a release.

A palindrome, compared case insensitively after trimming whitespace, earns 10000 points. Anything else is unsolved.

## fib-golf

The task asks for an x86-64 ELF `fibonacci` that prints the first `N` Fibonacci numbers, space separated, for `N` in `[0, 47]` given as the first argument. The smaller the binary the better.

The scorer runs the binary for every `N` and compares the output. A binary of 32 KiB or more is rejected. A correct binary earns points by its size. They fall off as `e^(-(bytes - 128) / 1500)`, scaled so that 128 bytes earn 10000 and 16 KiB earn 0:

![fib-golf points by ELF size](fib-golf-points.svg)

## r2wars

Two games, `r2wars-x86-32` and `r2wars-gb`, one per architecture. The task asks for `warrior.<architecture>.asm`, a warrior for [r2wars](https://github.com/radareorg/r2wars), the Core War on radare2's ESIL emulator. Two warriors share a 1 KiB arena and take turns executing instructions until one crashes.

The scorer assembles the warrior with `rasm2` for the architecture and applies the loading rules of r2wars: it has to assemble to 1 to 512 bytes. An accepted warrior earns 10000 points. The fights are the tournament of the multi player games and not scored yet.

Both games play on the image of `games/r2wars/Dockerfile`, which builds r2wars from a pinned commit plus `r2wars-headless.patch`, adding `r2wars --fight a.asm b.asm`, a combat on the console without the web server, and installs it with radare2 6.2.0 and the .NET runtime. The sandbox holds the two READMEs of r2wars and two small warriors per architecture under `/opt/r2wars/examples/<architecture>/`, the x86 ones from r2wars and the Game Boy ones from `games/r2wars/examples`, and none of the tournament entries.
