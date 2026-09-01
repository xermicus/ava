# Data Model

Models and harnesses are declared in `registry.json` at the repository root. A harness additionally needs an image directory under `agents` and an adapter in the `registry` module.

- A model is a name paired with one route per backend, each carrying the identifier and the token limits that backend expects.
- A harness is a name paired with the backends it can reach, listed most direct first.
- Pairing a harness with a model resolves to an invocation: the environment, arguments and configuration files handed to the container.
- A game is a task folder under `games` and a scorer of the same name implementing the `Game` trait in the `ava-game` crate.
