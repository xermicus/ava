# Games

A game is a folder under `games`: one task folder per turn holds what the agent gets, `task/` for a game with one turn, and `scorer.rs` implements the `Game` trait and is compiled into the `ava-game` crate, whose package is the `games` folder itself. A game needing software beyond the base image names a folder under `games` in `image()`, whose `Dockerfile` layers that software over the harness image and the scorer image, taking the image it extends as the build argument `BASE`.

The trait separates what is recorded from what is derived:

- `turns()` lists the turns of the game in order, at least one, each a task folder and the file the task asks for, which is what a passing push leaves behind as its entry.
- `inputs()` says what a seat gets before a turn from the seats it meets: entries of earlier turns, under the names the task text uses. The tournament seeds them into the workspace and mounts them for the verifier. A game with one turn says nothing.
- `verify()` runs in the scoring container on every push and reaches the verdict of the turn, passed or failed with the reason, naming the inputs the submission defeated when the turn is played against the entries of other seats. It records a fact and pays no points.
- `prepare()` computes the data every verification needs, once when the scoring container starts, so no push has to compute it. The default does nothing.
- `points()` ranks an entry within 0 and 10000 from the file and the verdict of the push that left it, without executing anything, wherever standings are shown. A game with nothing to rank beyond passing ranks nothing, and its entries show no points.
- `outcome()` reads how two seats came out of a round from what the round recorded, their entries and their verdicts, as a tally from the view of the first. Unless the game says otherwise the entries of the last turn are compared by their points: more points win, equal points draw, a missing entry forfeits. A game answering nothing needs a fight.
- `fight()` plays one pairing for such a game and tallies the rounds from the view of the first entry.

The agent submits by pushing the `task` branch. The verifier grades the files on it, and the entry of a passing push is kept under `runs/<run>/entries/<seconds>/`.

The tasks asking for a binary ask for an x86-64 Linux ELF whatever the host runs on, so entries compare across hosts. On a host of another architecture the verifier runs the binary through `qemu-x86_64` with the x86-64 libraries, and the sandbox carries the same emulator with `nasm` and the `x86_64-linux-gnu` cross toolchain. The architecture of the host is recorded on the run.

## Adding a game

1. Write the task of every turn, `games/<name>/task/task.md` for a game with one turn, and any files the agent needs.
2. Implement the `Game` trait in `games/<name>/scorer.rs`: the turns, the verifier, and the points curve when passing is not the whole story.
3. Declare the module by its path in `games/src/lib.rs` and add the implementation to the `GAMES` constant.
4. If the base image lacks software the game needs, return a folder under `games` from `image()` and write its `Dockerfile`, starting with `ARG BASE` and `FROM ${BASE}`. The build context is the `games` folder.
5. For a game whose entries fight each other, answer nothing from `outcome()` and implement `fight()`. For a game of several turns, list them in `turns()`, say in `inputs()` what a later turn gets from the other seats, verify every turn, and settle a pairing in `outcome()` from the verdicts. The tournaments do the rest.
6. If every verification needs the same data, compute it in `prepare()`.

A `cover.png`, `cover.svg`, `cover.webp` or `cover.jpg` in the game folder is the cover of its card on the games page. Without one the card shows the entry of record.

## crackme

A game of two turns: `defend/` asks for a crackme with its keygen, `attack/` asks for one keygen for the crackmes of all the other seats. A keygen turns a number into a key, and a crackme, given a number and a key, accepts the key its keygen makes for that number and nothing else. Because the crackme checks the key against the number, a key found for one number opens nothing for another, and a solver has to reproduce the function rather than collect accepted keys.

The defend task asks for two x86-64 ELF binaries: `keygen`, which started with an unsigned 64-bit decimal number prints one key of 20 to 256 printable ASCII characters without whitespace, the same key for the same number and a different key for a different number, and `crackme`, which started with a number and a key exits 0 when the key is the keygen's key for that number and 1 otherwise. Both answer within 10 seconds and stay under 1 MiB. The verifier runs a sample of random numbers through the keygen twice, checks the keys are stable and distinct, feeds each number with its key to the crackme, then feeds the crackme pairs it must refuse: a number with an empty key, with `password`, with its key altered in the first character, and with the key of another sampled number. The entry is the crackme alone: the keygen stays secret.

The attack task starts with the crackmes of the other seats in the workspace as `crackme.<seat>` and asks for one `keygen`, started with the seat and the number, under the same rules. The verifier runs a sample per crackme through the submitted keygen and feeds each number with its key to that crackme, and records the crackmes cracked; a push passes when it cracked one. The game ranks nothing beyond passing; its point is the tournament, where a pairing is two rounds, one per direction, each won by cracking the other's crackme, or by forfeit when the other left none. The attack turn only starts from a tournament.

The verifier trusts neither binary. Both run as the sandbox user in the scoring container, which has no network, under the 5 second timeout with their output capped, and confined by Landlock to reading and executing the system directories and their own file. Every process they start inherits the confinement. So the keygen cannot run the crackme beside it as an oracle, nor leave a helper behind that could, and the crackme cannot read the keygen it is asked to accept and refuse every keygen but its author's. The sample of numbers is drawn fresh for every verification, so neither binary can be a table of it. The solver may run the crackme in its own sandbox as often as it likes, which is the reverse engineering the game asks for. It cannot pass without a keygen that answers the numbers the verifier picks. A check weak enough to be inverted inside 5 seconds is a weak crackme, not a hole in the game.

## sanity-check

The task asks for a file `palindrome` holding an ASCII palindrome of at most 20 characters, then a release.

A palindrome, compared case insensitively after trimming whitespace, passes. Anything else fails. Passing is all there is to rank.

## fib-golf

The task asks for an x86-64 ELF `fibonacci` that prints the first `N` Fibonacci numbers, space separated, for `N` in `[0, 47]` given as the first argument, in at most 16 KiB. The smaller the binary the better.

The verifier runs the binary for every `N` and compares the output. A binary printing the right thing at more than 16 KiB fails with a reason saying so. A passing entry ranks by its size. The points fall off as `e^(-(bytes - 128) / 1500)`, scaled so that 128 bytes earn 10000 and 16 KiB earn 0:

![fib-golf points by ELF size](fib-golf-points.svg)

## chess-vm

The task asks for `bot.cvm`, a chess move scoring policy in the CVM assembly language. Every push reports the rating the policy achieved.

The CVM has 16 registers, a read only data section, no writable memory and no stack, plus chess instructions that read the position and the move being scored. `games/chess-vm/task/SPEC.md` specifies the language and is given to the agent. The engine implements the rules of chess: for every position it runs the program once per legal move and plays the move with the highest score, breaking ties with a seeded random generator. A submission therefore scores a single move and cannot search. The assembler rejects `.search` in a submission and limits a submission to 256 instructions, 1024 data words and 20 million cycles per decision.

The field is the 27 opponents under `games/chess-vm/opponents`, each a program in the same language, ranging from one that scores every move equally to a four ply search. Every opponent is written in the CVM, so a strategy that cannot be expressed in it means the language lacks an instruction. The instruction, data and cycle limits and the rejection of `.search` apply to submissions only, which is why the searching opponents can exist. A seeded round robin of six games per pair rates the opponents: gradient ascent fits ELO ratings to the results, then shifts them so the weakest opponent has rating 0. The round robin depends on nothing outside the build, so every machine computes the same ratings, and the ratings are not stored in the repository.

The verifier plays a submission 20 games against each opponent, alternating colours. Its rating is the rating whose expected score against the field equals the score it achieved. A program that faults or exhausts its cycle budget without producing a score fails the push, whatever its rating, because the engine then plays the first legal move for it; the failure reason contains the first fault and its line number. A program that produces a score for every decision passes, and the reason of the passing verdict contains its rating and the ratings of the weakest and strongest opponent. `points()` may not execute a program, so the game ranks nothing beyond passing.

The round robin takes minutes of processor time; playing one submission against the field takes seconds. `prepare` therefore runs the round robin when the scoring container starts and writes the ratings to a file in the temporary directory, which every later push reads. The file name contains a hash of the opponents, the round robin settings and the executable, so another build does not read it. If the file is missing, the next push runs the round robin again.

The verdict records the rating and the entry ranks at it in points, capped at 10000, so the best rated push is the entry of record. The records do not settle a pairing, so `outcome()` returns nothing and the scorer fights the entries instead. One combat is two games, one with each colour, and each game counts as one round for the first entry. [Tournaments](tournaments.md) run the fights.

The seeds are fixed, so the same program always plays the same games. The agent can see every seed: the verifier runs in CI, and every push reports the rating of the program it verified. A first policy is a few instructions; improving it means measuring one change at a time against the field, which takes hours. The default budget of 300 seconds is too short, so run this game with `-t`.

`cargo test --release -p ava-game -- --ignored` runs the round robin and compares the ratings against the recorded ones, verifies the reference bot in `games/chess-vm/reference.cvm` against the field, and runs the deep move generator counts. It takes about ten seconds on a machine with many cores, and minutes without `--release`.

## r2wars

Two games, `r2wars-x86-32` and `r2wars-gb`, one per architecture. The task asks for `warrior.<architecture>.asm`, a warrior for [r2wars](https://github.com/radareorg/r2wars), the Core War on radare2's ESIL emulator. Two warriors share a 1 KiB arena and take turns executing instructions until one crashes.

The verifier assembles the warrior with `rasm2` for the architecture and applies the loading rules of r2wars: it has to assemble to 1 to 512 bytes. A warrior that assembles passes. Passing is all there is to rank on its own, the fights are what tells warriors apart.

The game has one turn and nothing in the records settles a pairing, so the scorer fights the entries. A fight is the combats the tournament fixed, five unless chosen otherwise, of `r2wars --fight` between two entries, each best of three rounds on random load positions, staged as `first.<architecture>.asm` and `second.<architecture>.asm` because r2wars names a warrior by its file and reads the architecture out of the name. Every round the combats print is tallied: a round the first warrior wins, a round it loses, or a timeout, which is a draw. The fights are played by [tournaments](tournaments.md).

A round is capped at 50000 cycles, `MAX_CYCLES` in the patch, up from the 2000 of upstream r2wars. A cycle is one turn of one warrior, and an instruction costing several cycles takes as many turns. Two warriors looping past each other reach the cap, the round is a timeout, tallied as a draw and replayed from fresh positions without counting as one of the three, and the third timeout of a combat ends it as a draw. A timed out round takes about 18 seconds, a fight of five combats between two warriors that never meet about 280, under the wall clock of 600 seconds every fight runs under, `FIGHT_TIMEOUT_SECONDS` in `ava-run`; a fight that outlives it fails as described under [the pairings](tournaments.md#the-pairings).

Both games play on the image of `games/r2wars/Dockerfile`, which builds r2wars from a pinned commit plus `r2wars-headless.patch`, adding `r2wars --fight a.asm b.asm`, a combat on the console without the web server, and installs it with radare2 6.2.0 and the .NET runtime. The sandbox holds the two READMEs of r2wars and two small warriors per architecture under `/opt/r2wars/examples/<architecture>/`, the x86 ones from r2wars and the Game Boy ones from `games/r2wars/examples`, and none of the tournament entries.
