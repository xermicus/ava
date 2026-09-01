FROM ava/host-env AS builder

COPY --chown=1000:1000 Cargo.toml Cargo.lock /home/agent/scorer/
COPY --chown=1000:1000 crates /home/agent/scorer/crates

RUN cargo install --path /home/agent/scorer/crates/ava --locked

# The runtime carries what serving and scoring need: git for the remote and
# the hooks, python3 for the hook reshaping, libstdc++ for submissions, and
# nothing to build with. The uid is pinned to what the workspace is mounted with.
FROM ubuntu:26.04@sha256:2260313b31c8c011cd2eebe728008efac1b3982be73eb71348ea2648d2c0e09b

RUN apt update && \
	apt install -y --no-install-recommends git python3 libstdc++6 && \
	rm -rf /var/lib/apt/lists/* && \
	userdel -r ubuntu 2>/dev/null || true && \
	useradd --uid 1000 -m -s /bin/bash agent

COPY --from=builder /home/agent/.cargo/bin/ava /usr/local/bin/ava
COPY --chown=1000:1000 scorer/entry.sh /home/agent/score-entry.sh
COPY --chown=1000:1000 scorer/hooks /home/agent/hooks

ENTRYPOINT ["ava"]
