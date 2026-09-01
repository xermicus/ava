FROM ava/host-env

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV PATH="${AGENT_HOME}/.opencode/bin:${PATH}"

RUN curl -fsSL https://opencode.ai/install | VERSION=1.18.23 bash

RUN mkdir -p ${AGENT_HOME}/.config/opencode/plugin && \
	printf '{"dependencies":{"@opencode-ai/plugin":"1.18.23"}}' > ${AGENT_HOME}/.config/opencode/package.json && \
	npm_config_offline=false npm install --omit=dev --no-audit --no-fund --prefix ${AGENT_HOME}/.config/opencode && \
	chown -R 1000:1000 ${AGENT_HOME}/.config

ENV npm_config_offline=true

COPY --chown=1000:1000 --chmod=755 opencode-loop /usr/local/bin/opencode-loop

ENTRYPOINT ["ava-bridge", "opencode-loop"]
