# The scorer: ava built from the workspace, on a runtime carrying what serving
# and scoring need and nothing to build with. The game layer adds what a game's
# scorer runs.

FROM ubuntu:26.04@sha256:2260313b31c8c011cd2eebe728008efac1b3982be73eb71348ea2648d2c0e09b AS builder

ENV RUSTUP_HOME="/opt/rustup"
ENV CARGO_HOME="/opt/cargo"
ENV PATH="/opt/cargo/bin:${PATH}"

RUN apt update && \
	apt install -y --no-install-recommends ca-certificates curl build-essential && \
	rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
	--profile minimal \
	--default-toolchain 1.98.0

COPY Cargo.toml Cargo.lock /scorer/
COPY crates /scorer/crates
COPY games /scorer/games

RUN cargo install --path /scorer/crates/ava --locked --root /usr/local

# git for the remote and the hooks, python3 for the hook reshaping, libstdc++
# for submissions, and the emulator running the x86-64 binaries of the tasks
# on a host of another architecture.
FROM ubuntu:26.04@sha256:2260313b31c8c011cd2eebe728008efac1b3982be73eb71348ea2648d2c0e09b

# The uid is pinned to what the workspace is mounted with.
RUN apt update && \
	apt install -y --no-install-recommends git python3 libstdc++6 && \
	if [ "$(dpkg --print-architecture)" != "amd64" ]; then \
		apt install -y --no-install-recommends qemu-user libc6-amd64-cross libstdc++6-amd64-cross; \
	fi && \
	rm -rf /var/lib/apt/lists/* && \
	userdel -r ubuntu 2>/dev/null || true && \
	useradd --uid 1000 -m -s /bin/bash agent

COPY --from=builder /usr/local/bin/ava /usr/local/bin/ava
COPY --chown=1000:1000 scorer/entry.sh /home/agent/score-entry.sh
COPY --chown=1000:1000 scorer/hooks /home/agent/hooks

ENTRYPOINT ["ava"]
