# The image of the tcc-opt game: the pinned tinycc tree as the scaffold, a
# bootstrap tcc built from it to build the submissions with, the sources of the
# test suites with the deflate fixture, and the tools the suites build with.
ARG BASE

FROM ubuntu:26.04@sha256:2260313b31c8c011cd2eebe728008efac1b3982be73eb71348ea2648d2c0e09b AS toolchain

ARG TINYCC_COMMIT="0fb54300b56512754221d80adda85ddb9815bceb"
ARG ZLIB_VERSION="1.3.1"
ARG ZLIB_SHA256="9a93b2b7dfdac77ceba5a558a580e74667dd6fede4585b91eefb60f03b72df23"
ARG SQLITE_RELEASE="3500400"
ARG SQLITE_SHA256="b7b4dc060f36053902fb65b344bbbed592e64b2291a26ac06fe77eec097850e9"
ARG LIBPNG_VERSION="1.6.50"
ARG LIBPNG_SHA256="708f4398f996325819936d447f982e0db90b6b8212b7507e7672ea232210949a"

RUN apt update && \
	apt install -y --no-install-recommends ca-certificates curl git build-essential unzip python3 && \
	rm -rf /var/lib/apt/lists/*

RUN git init --quiet /opt/scaffold/tinycc && \
	git -C /opt/scaffold/tinycc fetch --quiet --depth 1 https://repo.or.cz/tinycc.git "${TINYCC_COMMIT}" && \
	git -C /opt/scaffold/tinycc checkout --quiet FETCH_HEAD && \
	rm -rf /opt/scaffold/tinycc/.git

RUN cp -r /opt/scaffold/tinycc /tmp/tinycc && \
	cd /tmp/tinycc && \
	./configure --prefix=/opt/tcc-opt/bootstrap && \
	make && \
	make install && \
	rm -rf /tmp/tinycc

RUN mkdir -p /opt/tcc-opt/suites && \
	cd /opt/tcc-opt/suites && \
	curl -fsSL -o zlib.tar.gz "https://github.com/madler/zlib/releases/download/v${ZLIB_VERSION}/zlib-${ZLIB_VERSION}.tar.gz" && \
	echo "${ZLIB_SHA256}  zlib.tar.gz" | sha256sum --check --strict && \
	tar xf zlib.tar.gz && \
	mv "zlib-${ZLIB_VERSION}" zlib && \
	curl -fsSL -o sqlite.zip "https://www.sqlite.org/2025/sqlite-src-${SQLITE_RELEASE}.zip" && \
	echo "${SQLITE_SHA256}  sqlite.zip" | sha256sum --check --strict && \
	unzip -q sqlite.zip && \
	mv "sqlite-src-${SQLITE_RELEASE}" sqlite && \
	curl -fsSL -o libpng.tar.gz "https://download.sourceforge.net/libpng/libpng-${LIBPNG_VERSION}.tar.gz" && \
	echo "${LIBPNG_SHA256}  libpng.tar.gz" | sha256sum --check --strict && \
	tar xf libpng.tar.gz && \
	mv "libpng-${LIBPNG_VERSION}" libpng && \
	rm -f zlib.tar.gz sqlite.zip libpng.tar.gz && \
	cp -r /opt/scaffold/tinycc tinycc

COPY --chmod=755 tcc-opt/check /opt/tcc-opt/
COPY tcc-opt/deflate-fixture.c /opt/tcc-opt/

RUN mkdir /opt/tcc-opt/suites/corpus && \
	cd /opt/tcc-opt/suites/corpus && \
	cat ../sqlite/src/*.c | head -c 1000000 > text && \
	printf 'hello, deflate\n' > short && \
	yes ab | tr -d '\n' | head -c 1000000 > repeat && \
	python3 -c 'import random, sys; random.seed(1); sys.stdout.buffer.write(random.randbytes(400000))' > random && \
	/opt/tcc-opt/check /opt/tcc-opt/bootstrap zlib record

FROM ${BASE}

COPY --from=toolchain /opt/tcc-opt /opt/tcc-opt
COPY --from=toolchain /opt/scaffold /opt/scaffold

USER root
RUN apt update && \
	apt install -y --no-install-recommends binutils libc6-dev make tcl tcl-dev && \
	apt clean && \
	rm -rf /var/lib/apt/lists/*

# The baseline the game scores against: the code size the unmodified bootstrap
# compiler gives each suite, measured once here so no push has to.
RUN for suite in zlib sqlite libpng tinycc; do \
		/opt/tcc-opt/check /opt/tcc-opt/bootstrap "$suite" build; \
	done > /opt/tcc-opt/reference.txt && cat /opt/tcc-opt/reference.txt

USER agent
