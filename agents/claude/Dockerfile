FROM ava/host-env

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV PATH="${AGENT_HOME}/.claude/bin:${PATH}"

RUN curl -fsSL https://claude.ai/install.sh | bash -s 2.1.247

RUN claude plugin marketplace add anthropics/claude-code && \
	claude plugin install ralph-wiggum@claude-code-plugins

ENTRYPOINT ["ava-bridge", "claude", "--dangerously-skip-permissions"]
