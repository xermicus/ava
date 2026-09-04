FROM ava/base

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN npm install -g --ignore-scripts --prefix ${AGENT_HOME}/.local @openai/codex@0.152.0

RUN mkdir -p ${AGENT_HOME}/.codex

ENTRYPOINT ["ava-bridge", "codex"]
