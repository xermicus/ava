# Games

A game is a folder under `games`: `task/` holds what the agent gets, `scorer.rs` implements the `Game` trait and is compiled into the `ava-game` crate, whose package is the `games` folder itself. A game needing software beyond the base image names a folder under `games` in `image()`, whose `Dockerfile` layers that software over the harness image and the scorer image, taking the image it extends as the build argument `BASE`.

The trait separates what is recorded from what is derived:

- `entry()` names the file the task asks for. It is what a passing push leaves behind as its entry.
- `verify()` runs in the scoring container on every push and reaches a verdict, passed or failed with the reason. It records a fact and pays no points.
- `points()` ranks an entry within 0 and 10000 from the file alone, without executing it, wherever standings are shown. A game with nothing to rank beyond passing pays the maximum for every passing entry.
- `playout()` says how entries meet: alone against the ceiling of the game, where a tournament compares them by their points; automated, where the scorer fights two entries without an agent; or played, naming the game whose runs attack an entry.
- `fight()` plays one pairing for an automated game and tallies the rounds from the view of the first entry.
- `verify()` of a game that attacks an entry gets that entry as its challenge, the file mounted into the scoring container and seeded into the attacker's workspace.

The agent submits by pushing the `task` branch. The verifier grades the files on it, and the entry of a passing push is kept under `runs/<run>/entries/<seconds>/`.

## Adding a game

1. Write `games/<name>/task/task.md` and any files the agent needs.
2. Implement the `Game` trait in `games/<name>/scorer.rs`: the entry file, the verifier, and the points curve when passing is not the whole story.
3. Declare the module by its path in `games/src/lib.rs` and add the implementation to the `GAMES` constant.
4. If the base image lacks software the game needs, return a folder under `games` from `image()` and write its `Dockerfile`, starting with `ARG BASE` and `FROM ${BASE}`. The build context is the `games` folder.
5. For a game whose entries fight each other, return `Playout::Automated` and implement `fight()`. For a game whose entries are attacked by agents, write the attacking game as a second folder with its own `task/`, verify its submissions against the challenge, and name it in `Playout::Played`. The tournaments do the rest.

## crackme

Two games in one pair, `crackme` and `crackme-solve`, the first with a played playout naming the second.

The authoring task asks for an x86-64 ELF `crackme` that prints `OK` and exits 0 when started with the secret key as its only argument and exits 1 for any other key, within 5 seconds and 1 MiB, plus `key`, the secret as one line of at most 256 printable ASCII characters without whitespace. The verifier runs the crackme with the key, then with an empty key, with `password` and with the key changed in its first character, and passes it when the key is the only one accepted. The entry is the binary alone: the key is never kept.

The solving task starts with the crackme of another seat in the workspace and asks for `key`, a key it accepts. The verifier runs the original crackme, mounted as the challenge, with the submitted key. A passing entry ranks at the maximum in both games; the point of the pair is the tournament, where every seat's crackme is attacked by every other seat.

## sanity-check

The task asks for a file `palindrome` holding an ASCII palindrome of at most 20 characters, then a release.

A palindrome, compared case insensitively after trimming whitespace, passes. Anything else fails. A passing entry ranks at the maximum.

## fib-golf

The task asks for an x86-64 ELF `fibonacci` that prints the first `N` Fibonacci numbers, space separated, for `N` in `[0, 47]` given as the first argument, in at most 16 KiB. The smaller the binary the better.

The verifier runs the binary for every `N` and compares the output. A binary printing the right thing at more than 16 KiB fails with a reason saying so. A passing entry ranks by its size. The points fall off as `e^(-(bytes - 128) / 1500)`, scaled so that 128 bytes earn 10000 and 16 KiB earn 0:

![fib-golf points by ELF size](fib-golf-points.svg)

## r2wars

Two games, `r2wars-x86-32` and `r2wars-gb`, one per architecture. The task asks for `warrior.<architecture>.asm`, a warrior for [r2wars](https://github.com/radareorg/r2wars), the Core War on radare2's ESIL emulator. Two warriors share a 1 KiB arena and take turns executing instructions until one crashes.

The verifier assembles the warrior with `rasm2` for the architecture and applies the loading rules of r2wars: it has to assemble to 1 to 512 bytes. A warrior that assembles passes, and ranks at the maximum on its own.

The playout is automated. A fight is one combat of `r2wars --fight` between two entries, staged as `first.<architecture>.asm` and `second.<architecture>.asm` because r2wars names a warrior by its file and reads the architecture out of the name. Every round the combat prints is tallied: a round the first warrior wins, a round it loses, or a timeout, which is a draw. The fights are played by [tournaments](tournaments.md).

Both games play on the image of `games/r2wars/Dockerfile`, which builds r2wars from a pinned commit plus `r2wars-headless.patch`, adding `r2wars --fight a.asm b.asm`, a combat on the console without the web server, and installs it with radare2 6.2.0 and the .NET runtime. The sandbox holds the two READMEs of r2wars and two small warriors per architecture under `/opt/r2wars/examples/<architecture>/`, the x86 ones from r2wars and the Game Boy ones from `games/r2wars/examples`, and none of the tournament entries.
