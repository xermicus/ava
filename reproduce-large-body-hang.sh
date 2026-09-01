#!/bin/bash

# Further defects observed on llm.substrate.dev on 2026-08-28, from 170 streaming
# requests made by benchmark runs:
#
# - 6 requests were cut mid stream, 4 of them between 18:21 and 18:25 across two
#   runs and two models, while the 141 requests before 18:20 were clean. nginx
#   logged "upstream prematurely closed connection" for each, so the gateway
#   closed first. Not load dependent: 16 requests completed while three other
#   streams were in flight.
# - /v1/models is stale. claude-sonnet-5 serves but is not listed, and the glm
#   listing ends at 5.1 while 5.2 and 5.3 both answer.
# - /model/info and /model_info know none of the models we use, so context and
#   output limits have to be taken from the serving provider instead.
# - deepseek-flash answers with x-litellm-attempted-fallbacks: 1 while the
#   self hosted deployment is down, which attributes a self hosted result to
#   openrouter. deepseek-flash-parity fails with
#   "Hosted_vllmException - Cannot connect to host host.docker.internal".
# - Every response carries two Server headers, Caddy and uvicorn, which nginx
#   warns about once per request.

set -euo pipefail

if [ $# -lt 2 ]; then
	echo "usage: $0 <base-url> <api-key> [model] [megabytes] [timeout-seconds]" >&2
	echo "example: $0 https://llm.example.invalid sk-key kimi-k3 3 60" >&2
	exit 2
fi

base="$1"
key="$2"
model="${3:-kimi-k3}"
megabytes="${4:-3}"
seconds="${5:-60}"

answered() {
	local code="$1"
	if [ -z "${code}" ] || [ "${code}" = "000" ]; then
		echo "none"
	else
		echo "${code}"
	fi
}

probe() {
	answered "$(curl -sS -m 20 -o /dev/null -w "%{http_code}" \
		-H "x-api-key: ${key}" "${base}/v1/models" 2>/dev/null || true)"
}

body="$(mktemp)"
trap 'rm -f "${body}"' EXIT

{
	printf '{"model":"%s","max_tokens":8,"messages":[{"role":"user","content":"' "${model}"
	head -c "$(( megabytes * 1000 * 1000 ))" /dev/zero | tr '\0' 'x'
	printf '"}]}'
} > "${body}"

echo "target:            ${base}"
echo "body:              $(wc -c < "${body}") bytes for model ${model}"

before="$(probe)"
echo "baseline models:   ${before}"

started="$(date +%s)"
large="$(answered "$(curl -sS -m "${seconds}" -o /dev/null -w "%{http_code}" \
	-H "x-api-key: ${key}" \
	-H "content-type: application/json" \
	-H "anthropic-version: 2023-06-01" \
	--data-binary "@${body}" \
	"${base}/v1/messages" 2>/dev/null || true)")"
echo "large request:     ${large} after $(( $(date +%s) - started ))s"

after="$(probe)"
echo "models after:      ${after}"

sleep "${seconds}"

recovered="$(probe)"
echo "models after wait: ${recovered}"

if [ "${before}" = "200" ] && [ "${after}" != "200" ] && [ "${recovered}" != "200" ]; then
	echo "reproduced: the service answered before the large body and stopped after it"
	exit 1
fi

if [ "${before}" = "200" ] && [ "${after}" != "200" ]; then
	echo "reproduced and recovered: the service stopped answering after the large body and came back within ${seconds}s"
	exit 3
fi

if [ "${before}" != "200" ]; then
	echo "inconclusive: the service was already not answering before the large body"
	exit 2
fi

echo "not reproduced: the service still answers"
