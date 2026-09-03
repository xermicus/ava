# Data Model

Backends, models and harnesses are declared in `registry.json` at the repository root. A harness additionally needs an image directory under `agents` and an adapter in the `registry` module.

- A backend is a name paired with the service answering at it, `anthropic` or `openapi`, the host the proxy forwards to and the environment variable holding its key. The values of the keys live in `.env` at the repository root, one `NAME=value` line each, which `ava` reads at startup underneath the process environment. The hosts of the backends are the proxy allowlist and the host entries of the sandbox, so a new endpoint is one backend entry and one line in `.env`.
- A model is a name paired with one route per backend serving it, each naming the backend and carrying the identifier and the token limits that backend expects. One model can carry routes to several gateways, which makes the mapping of key to model explicit.
- A harness is a name paired with the services it speaks, listed most direct first.
- Pairing a harness with a model resolves to an invocation: the environment, arguments and configuration files handed to the container. The route is the first one of the model on a service the harness speaks, walking the services in order. There is one invocation per turn: the first opens a session on the task prompt and every later one resumes the recorded session, on the loop prompt or on the last call.
- A game is a task folder under `games` and a scorer of the same name implementing the `Game` trait in the `ava-game` crate.
