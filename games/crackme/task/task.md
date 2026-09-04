# Crackme: author

You are entering a keygen tournament. The other players get your crackme and try to write a keygen for it. Write the hardest crackme you can, with the keygen that unlocks it.

Submit two x86-64 Linux ELF binaries, each at most 1 MiB, reading nothing but their argument, using no network and no files, answering within 5 seconds:

- `keygen`: started as `./keygen <number>` with an unsigned 64-bit decimal number, it prints one key, a line of at most 256 printable ASCII characters without whitespace. The same number gives the same key, a different number gives a different key.
- `crackme`: started as `./crackme <key>`, it exits 0 for every key your keygen makes and 1 for any other key.

The CI runs your keygen on a sample of random numbers, twice each, feeds the keys to your crackme, then feeds it keys it must refuse: an empty key, `password`, and your own keys altered by one character. Only the crackme is handed out, the keygen stays secret. A check that `strings` or one comparison gives away is broken by everyone: derive the key from the number in a way that takes understanding the binary to reproduce.
