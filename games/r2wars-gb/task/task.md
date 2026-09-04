# r2wars: gb

You are entering an r2wars tournament, playing Game Boy. r2wars is a Core War on radare2's ESIL emulator: two warriors share a 1 KiB arena at address 0, are loaded at random positions and take turns executing one instruction each, slower instructions costing more turns. A warrior dies when it executes an invalid instruction or reads or writes outside the arena. The last one alive wins.

Submit `warrior.gb.asm`: the assembly of your warrior, as `rasm2 -a gb -b 16 -f warrior.gb.asm` assembles it, to at most 512 bytes. rasm2 assembles almost nothing for the Game Boy besides `jr`, so warriors are written as `.hex` lines of opcodes.

r2wars and radare2 6.2.0 are installed. `r2wars --fight warrior.gb.asm /opt/r2wars/examples/gb/<example>.asm` plays a combat and prints every round with how the loser died. `rasm2 -a gb -b 16 -f warrior.gb.asm` assembles and shows your bytes, `r2 malloc://1024` with `aei`, `aeim`, `wx`, `aer PC=` and `aes` emulates a warrior step by step. `/opt/r2wars/README.md` and `/opt/r2wars/python/README.md` hold the rules and a debugging guide, `/opt/r2wars/examples/gb/` two small warriors to spar with.

The CI checks that r2wars accepts your warrior. Write the strongest warrior you can: locate the opponent, overwrite it, survive.
