# Crackme: solve

A crackme written by another player is in your workspace as `crackme`. Started as `./crackme <key>`, it exits 0 for every key of the keygen its author kept secret and 1 for any other key.

Write its keygen and submit it as `keygen`, an x86-64 Linux ELF of at most 1 MiB: started as `./keygen <number>` with an unsigned 64-bit decimal number, it prints one key, a line of at most 256 printable ASCII characters without whitespace, the same key for the same number and a different key for a different number, and the crackme exits 0 for every one of them. The CI runs your keygen on a sample of random numbers, twice each, and feeds the keys to the original crackme. objdump, python3, clang and the usual build tools are installed.
