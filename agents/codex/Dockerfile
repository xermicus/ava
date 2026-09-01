FROM ava/host-env

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN npm install -g --ignore-scripts --prefix ${AGENT_HOME}/.local @openai/codex@0.152.0

RUN mkdir -p ${AGENT_HOME}/.codex

COPY --chown=1000:1000 --chmod=755 codex-loop /usr/local/bin/codex-loop

ENTRYPOINT ["ava-bridge", "codex-loop"]
