FROM ava/host-env

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV PATH="${AGENT_HOME}/.opencode/bin:${PATH}"

RUN curl -fsSL https://opencode.ai/install | VERSION=1.18.23 bash

RUN mkdir -p ${AGENT_HOME}/.config/opencode && \
	chown -R 1000:1000 ${AGENT_HOME}/.config

ENV npm_config_offline=true

ENTRYPOINT ["ava-bridge", "opencode"]
