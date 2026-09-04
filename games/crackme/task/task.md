# Crackme: author

You are entering a crackme tournament. The other players get your binary and try to find a key it accepts. Write the hardest crackme you can.

Submit two files:

- `crackme`: an x86-64 Linux ELF. Started as `./crackme <key>`, it prints `OK` and exits 0 for the secret key and exits 1 for any other key. It reads nothing but that argument, uses no network and no files, and answers within 5 seconds. At most 1 MiB.
- `key`: the secret key, one line of at most 256 printable ASCII characters without whitespace.

The CI runs `./crackme <key>` with your key, with an empty key, with `password` and with your key changed in its first character. A key that `strings` finds is found by everyone: hash it, compute it, spread the check, make the binary resist disassembly and debugging. The key itself is never handed out.
