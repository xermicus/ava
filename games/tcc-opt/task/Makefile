# Build tcc into build/ and check it the way the CI does.

TCC_OPT := /opt/tcc-opt
PREFIX := $(CURDIR)/build
SUITES := zlib sqlite libpng tinycc

build:
	cd tinycc && ./configure --cc=$(TCC_OPT)/bootstrap/bin/tcc --prefix=$(PREFIX) && $(MAKE) && $(MAKE) install

quick-check: check-zlib

test: $(SUITES:%=check-%)

check-%: build
	TCC_OPT_SIZE=1 $(TCC_OPT)/check $(PREFIX) $*

.PHONY: build quick-check test
