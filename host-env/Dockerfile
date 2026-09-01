# The build configuration

FROM ubuntu:26.04@sha256:2260313b31c8c011cd2eebe728008efac1b3982be73eb71348ea2648d2c0e09b

ARG SANDBOX_USERNAME="agent"

ENV DEBIAN_FRONTEND="noninteractive"
ENV LANG=en_US.UTF-8
ENV LANGUAGE=en_US:en
ENV LC_ALL=en_US.UTF-8

SHELL ["/bin/bash", "-o", "pipefail", "-c"]


# The build dependencies

RUN apt update && \
	apt upgrade -y && \
	apt install -y --no-install-recommends \
	build-essential \
	ca-certificates \
	git \
	python3 \
	make \
	cmake \
	clang \
	ripgrep \
	wget \
	npm \
	curl \
	sudo \
	less \
	locales \
	pkg-config \
	golang-go \
	jq \
	socat \
	xxd

# The uid is pinned to what the workspace tmpfs is mounted with.
RUN sed -i -E "s/ ALL$/ NOPASSWD: ALL/g" /etc/sudoers && \
	userdel -r ubuntu 2>/dev/null || true && \
	useradd --uid 1000 -m -s /bin/bash ${SANDBOX_USERNAME} && \
	usermod -a -G sudo ${SANDBOX_USERNAME}

RUN sed -i -e 's/# en_US.UTF-8 UTF-8/en_US.UTF-8 UTF-8/' /etc/locale.gen && \
	locale-gen

COPY ava-bridge /usr/local/bin/ava-bridge

RUN chmod 755 /usr/local/bin/ava-bridge

# The toolchains shared by every agent

USER ${SANDBOX_USERNAME}
WORKDIR /home/${SANDBOX_USERNAME}

ENV AGENT_HOME="/home/${SANDBOX_USERNAME}"
ENV PATH="${AGENT_HOME}/.local/bin:${AGENT_HOME}/.cargo/bin:${AGENT_HOME}/.foundry/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.98.0

RUN mkdir -p ${AGENT_HOME}/workspace

CMD ["/bin/bash"]
