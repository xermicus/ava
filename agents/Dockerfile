# The base of every harness image: the sandbox user and the toolchains.

FROM ubuntu:26.04@sha256:2260313b31c8c011cd2eebe728008efac1b3982be73eb71348ea2648d2c0e09b

ENV DEBIAN_FRONTEND="noninteractive"
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt update && \
	apt upgrade -y && \
	apt install -y --no-install-recommends \
	build-essential \
	ca-certificates \
	clang \
	cmake \
	curl \
	git \
	golang-go \
	jq \
	less \
	locales \
	make \
	nasm \
	npm \
	pkg-config \
	python3 \
	ripgrep \
	socat \
	sudo \
	wget \
	xxd && \
	if [ "$(dpkg --print-architecture)" != "amd64" ]; then \
		apt install -y --no-install-recommends \
		binutils-x86-64-linux-gnu \
		g++-x86-64-linux-gnu \
		gcc-x86-64-linux-gnu \
		libc6-dev-amd64-cross \
		qemu-user; \
	fi && \
	apt clean && \
	rm -rf /var/lib/apt/lists/*

# The uid is pinned to what the workspace tmpfs is mounted with.
RUN sed -i -E "s/ ALL$/ NOPASSWD: ALL/g" /etc/sudoers && \
	userdel -r ubuntu 2>/dev/null || true && \
	useradd --uid 1000 -m -s /bin/bash agent && \
	usermod -a -G sudo agent

RUN sed -i -e 's/# en_US.UTF-8 UTF-8/en_US.UTF-8 UTF-8/' /etc/locale.gen && \
	locale-gen

COPY ava-bridge ava-analyst /usr/local/bin/

RUN chmod 755 /usr/local/bin/ava-bridge /usr/local/bin/ava-analyst

USER agent
WORKDIR /home/agent

ENV AGENT_HOME="/home/agent"
ENV PATH="${AGENT_HOME}/.local/bin:${AGENT_HOME}/.cargo/bin:${PATH}"

# The minimal profile leaves out rust-docs, which is over half of the agent
# home on its own and useless to a sandbox with no browser and no network. The
# home is copied into a memory backed volume for every run, so its size is RAM.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
	--profile minimal \
	--component clippy,rustfmt \
	--default-toolchain 1.98.0

RUN mkdir -p ${AGENT_HOME}/workspace

CMD ["/bin/bash"]
