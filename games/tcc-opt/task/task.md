# Compiler optimizations: code size in tinycc

The `tinycc` directory holds the Tiny C Compiler at commit 0fb5430. Build code size optimizations into it. They are on when `TCC_OPT_SIZE=1` is in the environment and `-O` (any level) is on the command line. Today tcc parses `-O` into `s->optimize` (`libtcc.c`) and uses it for nothing but defining `__OPTIMIZE__`.

## How it is built and verified

`make build` in the root of the workspace configures and builds `tinycc` with the bootstrap compiler `/opt/tcc-opt/bootstrap/bin/tcc` and installs it under `build/`. The CI runs `make build` on your push, takes `build/bin/tcc` and `build/lib/tcc` and nothing else out of it, and builds and tests four programs with that compiler and `TCC_OPT_SIZE=1`, all compiled with `-O2`:

- zlib 1.3.1: `make test`, then a fixture of deflate output the compiled library has to reproduce byte for byte.
- sqlite 3.50.4: `testfixture test/veryquick.test`.
- libpng 1.6.50: `make check`, against the zlib the same compiler built.
- tinycc itself at the commit you started from: `make test`, which includes compiling itself three times over.

All four have to pass. The score is the file size of `minigzip`, `sqlite3`, `pngtest` and `tcc` as linked by your compiler with `TCC_OPT_SIZE=1`, each against the same program built by the unmodified upstream compiler, a baseline the CI measured once ahead of time. The four count equally, and halving all four earns everything. The push output reports the sizes.

`make quick-check` builds and runs the zlib check in a few seconds. `make test` runs all four suites with the switch on, which takes several minutes, and `make -j4 test` runs them in parallel. Run one suite by hand with `/opt/tcc-opt/check /opt/tcc-opt/bootstrap zlib` for the unmodified compiler or your `build` prefix for yours, and `/opt/tcc-opt/suites` holds the sources of the four programs. The check script and the suites live outside the workspace and are the same in the CI, which has no C compiler but yours: nothing else compiles the test programs there.

The CI takes several minutes per push. Run the push with a timeout well above that, or in the background with its output in a file.

The deliverable is the `tinycc` tree with your changes and the `Makefile` on the `task` branch.
