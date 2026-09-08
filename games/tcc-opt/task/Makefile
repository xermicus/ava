# Build tcc into build/ and check it the way the CI does.

TCC_OPT := /opt/tcc-opt
PREFIX := $(CURDIR)/build
SUITES := zlib sqlite libpng tinycc

build:
	cd tinycc && ./configure --cc=$(TCC_OPT)/bootstrap/bin/tcc --prefix=$(PREFIX) && $(MAKE) -j$(shell nproc) && $(MAKE) install

quick-check: check-zlib

# Build once, then run the four suites at once. nproc is the cores this run was
# given, so the suites fill them without over-subscribing.
test: build
	$(MAKE) $(SUITES:%=check-%) -j$(words $(SUITES))

check-%: build
	TCC_OPT_SIZE=1 $(TCC_OPT)/check $(PREFIX) $*

.PHONY: build quick-check test
