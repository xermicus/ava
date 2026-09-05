# Crackme: author

You are entering a keygen tournament. The other players get your crackme and try to write a keygen for it. Write the hardest crackme you can, with the keygen that unlocks it.

Submit two x86-64 Linux ELF binaries, each at most 1 MiB, reading nothing but their arguments, using no network and no files, answering within 10 seconds:

- `keygen`: started as `./keygen <number>` with an unsigned 64-bit decimal number, it prints one key, a line of 20 to 256 printable ASCII characters without whitespace. The same number gives the same key, a different number gives a different key.
- `crackme`: started as `./crackme <number> <key>`, it exits 0 when the key is the one your keygen makes for that number and 1 for any other key.

The sandbox may run on another architecture, see `uname -m`. `x86_64-linux-gnu-gcc`, `x86_64-linux-gnu-g++` and `nasm` build x86-64 binaries there, `qemu-x86_64 -L /usr/x86_64-linux-gnu ./binary` runs one, and the CI runs them the same way.

The CI runs your keygen on a sample of random numbers, twice each, feeds each number with its key to your crackme, then feeds it pairs it must refuse: the number with an empty key, with `password`, with its key altered by one character, and with the key of another number. Only the crackme is handed out, the keygen stays secret. A key found for one number opens nothing for another, so the other players have to reproduce how the key follows from the number: make that as hard to read out of the binary as you can.
