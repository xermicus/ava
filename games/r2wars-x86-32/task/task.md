# r2wars: x86-32

You are entering an r2wars tournament, playing x86 32 bit. r2wars is a Core War on radare2's ESIL emulator: two warriors share a 1 KiB arena at address 0, are loaded at random positions and take turns executing one instruction each, slower instructions costing more turns. A warrior dies when it executes an invalid instruction or reads or writes outside the arena. The last one alive wins.

Submit `warrior.x86-32.asm`: the assembly of your warrior, as `rasm2 -a x86 -b 32 -f warrior.x86-32.asm` assembles it, to at most 512 bytes.

r2wars and radare2 6.2.0 are installed. `r2wars --fight warrior.x86-32.asm /opt/r2wars/examples/x86-32/<example>.asm` plays a combat and prints every round with how the loser died. `rasm2 -a x86 -b 32 -f warrior.x86-32.asm` assembles and shows your bytes, `r2 malloc://1024` with `aei`, `aeim`, `wx`, `aer PC=` and `aes` emulates a warrior step by step. `/opt/r2wars/README.md` and `/opt/r2wars/python/README.md` hold the rules and a debugging guide, `/opt/r2wars/examples/x86-32/` two small warriors to spar with.

The CI checks that r2wars accepts your warrior. Write the strongest warrior you can: locate the opponent, overwrite it, survive.
