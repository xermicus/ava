FROM ava/base

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN npm install -g --ignore-scripts --prefix ${AGENT_HOME}/.local @earendil-works/pi-coding-agent@0.84.3

RUN mkdir -p ${AGENT_HOME}/.pi/agent

ENTRYPOINT ["ava-bridge", "pi"]
