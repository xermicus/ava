# Data Model

## The registry

Backends, models and harnesses are declared in `registry.json` at the repository root, agents in `agents.json` beside it. A harness additionally needs an image directory under `agents` and an adapter in the `registry` module.

- A backend is a name paired with the service answering at it, `anthropic` or `openapi`, the host the proxy forwards to and the environment variable holding its key. The values of the keys live in `.env` at the repository root, one `NAME=value` line each, which `ava` reads at startup underneath the process environment. The hosts of the backends are the proxy allowlist and the host entries of the sandbox, so a new endpoint is one backend entry and one line in `.env`.
- A model is a name paired with one route per backend serving it, each naming the backend and carrying the identifier and the token limits that backend expects. One model can carry routes to several gateways, which makes the mapping of key to model explicit. The `context_window` of the route is the context every harness compacts within, described in [Running benchmarks](benchmarks.md). The `price` of a route is what the backend bills it at, in dollars per million input, output, cache read and cache write tokens; the cost of a run is its tokens at the price of its route, derived where it is shown, so a route without a price has runs without a cost.
- A harness is a name paired with the services it speaks, listed most direct first.
- An agent is a name paired with a harness, a model and the backend serving it, one entry of the list in `agents.json`, none when there is no such file. One agent at most is marked as the analyst, the one the analysis settings offer first. The name selects the pairing on the command line and in the forms of the web interface. It is letters, digits, dashes, underscores and dots, taken by no other agent and by no harness, and the harness serves the model at the backend. Without a backend the first route of the model serves. The agents page of the web interface adds and removes agents and the page of an agent changes it, writing the file whole. Renaming an agent changes nothing in the records, which hold the harness and the model.
- Pairing a harness with a model resolves to an invocation: the environment, arguments and configuration files handed to the container. The route is the one on the backend of the agent, else the first one of the model on a service the harness speaks, walking the services in order. There is one invocation per turn: the first opens a session on the task prompt and every later one resumes the recorded session, on the loop prompt or on the last call.
- A game is a folder under `games`, holding one task folder per turn, `task/` for a game with one turn, and the verifier implementing the `Game` trait of the `ava-game` crate in `scorer.rs`. A game needing software beyond the base image names the folder whose `Dockerfile` layers it over the harness image and the scorer image.

## The wire format

Everything the core produces and consumes is a record of the `ava-wire` crate, serialized as JSON. Every record carries the `version` of the format, and the items inside it carry the version of what they name. The records hold facts. Points, ratings and standings are derived from them wherever they are shown and never stored, so the knobs deriving them can change without touching a record.

- An agent is a harness paired with a model. This is the identity ratings key on. Everything else is a setting of a run: the thinking level, the backend serving the model, the version of the harness, which is only knowable once the image exists. A setup is an agent with the thinking level and the backend it plays under. A seat holds one, an analyst is one.
- A verdict is what the verifier of a game says about one submission: passed, the reason when it did not, for a turn played against the entries of other seats the inputs the submission defeated, by name, and for a game grading against a field the rating it measured. An attempt is one graded push, the verdict and the seconds it arrived at, counted from the start of the scoring container. Attempt lines from before the wire format spell the verdict `solved`, which reads as `passed`.
- A tally is the rounds of one pairing from the view of its first seat: won, drawn and lost. One shape covers an r2wars fight of several rounds and a single verdict pairing, where it reads 1-0-0 or 0-0-1.
- The run record is `runs/<run>/run.json`. It is written when the run starts with the harness and its version, the model, the thinking level, the backend serving the model and the id it is asked for there, the game and its version, the architecture of the host, the time budget, the image id, the prompt and the arguments, the names of the variables the sandbox was given without their values, the context window of its route, the name it was started under, the analyst it is due when it was started with one, the turn of the game it plays, and its inputs: the entries of other runs it got, each by run, attempt and the name it was seeded under. A record from before the turns holds the entry it attacked as its `challenge`, which reads as one input of the second turn. It is completed when the run is over with the second it finished, every attempt, the metrics aggregated from the proxy log and the compactions counted on the console of the harness. A record without an end is a run still going, or one that broke. A run from before the wire format left its metrics and its end in a separate `score.json`, which is read in their place.
- The analysis record is `runs/<run>/analysis.json`: the analyst, its harness version, the image, the seconds it was given, when it started and finished, its turns, the metrics of its requests, and its report, or the reason it failed. A record from before the report holds `analysis_summary` and `analysis` alone.
- The game version is the short hash of the last commit touching the game folder and the folder of its image, suffixed with `-dirty` when the working tree differs from it, and empty outside a repository. It names the verifier a verdict was reached under. Points are derived, so a changed curve re-ranks every entry rather than invalidating the runs before it.
- The tournament record is `tournaments/<name>/tournament.json`: the game and its version at creation, the pairing scheme, the seconds every run is given, the combats every fight plays, one for a record from before they were fixed, the analyst and its seconds when one was chosen, the lobby as the setup in every seat, and the rounds. A round holds when it started and finished, one entry per seat per turn naming the run it played and the seconds of the attempt that is its entry of record, and the pairings that took a fight in the order they were fought, each two seats, the second they were fought at, their tally and the reason when the tally is not the outcome of a fight. A record from before the turns holds its attacks as pairings naming the run that played them. Pairings the game settles from the records are not recorded, since the standings derive them. A round without an end is playing or broke off, and only finished rounds count for the standings.

The report, one or two sentences per field:

| field | content |
|---|---|
| `strategy` | what the agent set out to do and how |
| `went_well` | what worked |
| `agent_mistakes` | what the agent got wrong by its own doing |
| `environment_issues` | what broke around the agent: backend, harness, sandbox, verifier; empty when nothing did |
| `decisive` | the one thing that made or broke the outcome |
| `attribution` | one of `agent`, `environment`, `mixed` |
| `verification` | how the agent checked its work before pushing, or that it did not |
| `pacing` | where the budget went and when the first pass was banked, if ever |
| `counterfactual` | the smallest change that would have flipped the outcome |
| `failure_mode` | empty on a pass, else `never_wrote`, `unbanked`, `hung_tool`, `wrong_place`, `wrong_solution`, `environment` or `other` |
| `other_failure` | the failure in a few words when none of the modes fits |
| `summary` | a handful of sentences |
| `analysis` | the full analysis in markdown |

Two clocks appear in the records. The seconds of an attempt count from the start of the scoring container, which every attempt of a run shares and which an entry is named by. The seconds of a run and a tournament are epoch seconds.
