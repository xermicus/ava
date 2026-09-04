# The image of the r2wars games: radare2 and r2wars with the headless fight, and
# two example warriors per architecture.
ARG BASE

FROM ubuntu:26.04@sha256:2260313b31c8c011cd2eebe728008efac1b3982be73eb71348ea2648d2c0e09b AS r2wars

ARG RADARE2_VERSION="6.2.0"
ARG RADARE2_SHA256_AMD64="eb82324e83315887fbee6f5d8632c982c593e056a87180f1bec5ccb06c463aeb"
ARG RADARE2_SHA256_ARM64="e866525e9874588d478d536cca38cf9a7562896725efb4119b886101fd93f1ec"
ARG R2WARS_COMMIT="2121fed6871accaa6e24a711dba4c9d29a176459"

RUN apt update && \
	apt install -y --no-install-recommends ca-certificates curl git dotnet-sdk-10.0 && \
	ARCH=$(dpkg --print-architecture) && \
	case "$ARCH" in amd64) SHA256=$RADARE2_SHA256_AMD64 ;; arm64) SHA256=$RADARE2_SHA256_ARM64 ;; esac && \
	curl -fsSL "https://github.com/radareorg/radare2/releases/download/${RADARE2_VERSION}/radare2_${RADARE2_VERSION}_${ARCH}.deb" -o /radare2.deb && \
	echo "$SHA256  /radare2.deb" | sha256sum --check --strict

COPY r2wars/r2wars-headless.patch /r2wars-headless.patch

RUN git init --quiet /opt/r2wars && \
	git -C /opt/r2wars fetch --quiet --depth 1 https://github.com/radareorg/r2wars.git "${R2WARS_COMMIT}" && \
	git -C /opt/r2wars checkout --quiet FETCH_HEAD && \
	git -C /opt/r2wars apply /r2wars-headless.patch && \
	dotnet publish /opt/r2wars/csharp/r2wars.csproj --configuration Release --output /opt/r2wars/bin --no-self-contained

FROM ${BASE}

COPY --from=r2wars /radare2.deb /tmp/radare2.deb
COPY --from=r2wars /opt/r2wars/bin /opt/r2wars/bin
COPY --from=r2wars /opt/r2wars/README.md /opt/r2wars/README.md
COPY --from=r2wars /opt/r2wars/python/README.md /opt/r2wars/python/README.md
COPY --from=r2wars /opt/r2wars/examples/pancake.x86-32.asm /opt/r2wars/examples/jordi.x86-32.asm /opt/r2wars/examples/x86-32/
COPY r2wars/examples /opt/r2wars/examples
COPY r2wars/r2wars /usr/local/bin/r2wars

USER root
RUN apt update && \
	apt install -y --no-install-recommends aspnetcore-runtime-10.0 /tmp/radare2.deb && \
	apt clean && \
	rm -rf /var/lib/apt/lists/* /tmp/radare2.deb
USER agent
