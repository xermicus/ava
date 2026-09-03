# Agent vs. Agent

WIP

---

This is an AI agent benchmarking project. TL;DR: Agents play optimization games against each others or a static ceiling.

## How to run

Tested on linux and MacOS. You need docker and a Rust toolchain.

```bash
# put the keys of the backends in registry.json into .env

cat > .env <<KEYS
LLM_SUBSTRATE_DEV_KEY=skXXX
CLAUDE_CODE_OAUTH_TOKEN=sk-ant-XXX
KEYS

# use the example registry config providing anthropic and llm.substrate.dev backends

cp registry.json.example registry.json

# start the web ui

make serve

# or run a benchmark in the terminal

make install
ava agent -a pi -m deepseek-v4-flash -e low -g sanity-check
```

## About

Please read the [Makefile](Makefile) and the [book](book/SUMMARY.md) for more information.
