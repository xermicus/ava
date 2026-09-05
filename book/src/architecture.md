# Architecture

The architecture follows these principles:
- Benchmarks should be reproducible
- Benchmarks should be secure
  - Agents under test can't cheat
  - Evaluation of potentially untrusted code submissions is contained
- KISS - explainable to a fellow engineer in a few sentences

## Running benchmarks

A run pairs a model with a harness, lets that agent attempt a task and scores the submission.

### Components

`ava` manages the entire lifecycle, either from the CLI or web interface.

The agent as well as the task evaluation are sandboxed within a docker container.
The sandbox docker container has no network access.
A unix socket that connects to the `ava-proxy` sidecar is mounted in.
`ava-proxy` reverse-proxies to the LLM backend, allowing the agent to access LLMs.

The benchmark task is mounted into the containers.
The proxy routes the `git` host to a bare repository in a container without network, which is how an agent submits: every push to the `task` branch is verified on the spot and the entry of every passing push is kept. `ava remote` serves that repository over the score socket, a CGI shim around `git http-backend` answering one request at a time. The `score` host on the same socket verifies a posted tar without recording it.
The isolation rests on the containers sharing nothing but a volume of unix sockets: only the proxy has a network, so every byte leaving the sandbox or reaching the scorer passes through a socket the proxy serves.

Each game implements a verifier, for security reasons it's evaluated in a container too.
The verifier checks the submission however it wishes and records a verdict, never points.
`ava` collects the entries and the metrics from the side-car logs after the run.
The metrics also record which models were accessed through the proxy, exposing a run that used another model than the pre-configured one.

### Sequence diagram of a benchmark run

```text
  +-----+         +---------+      +-----------+    +--------+      +-------------+
  | ava |         | sandbox |      | ava-proxy |    | scorer |      | LLM backend |
  +-----+         +---------+      +-----------+    +--------+      +-------------+
     |                 |                 |               |                 |
     | start proxy     |                 |               |                 |
     |----------------------------------->               |                 |
     | start scorer    |                 |               |                 |
     |--------------------------------------------------->                 |
     | start agent     |                 |               |                 |
     |----------------->                 |               |                 |
     |                 |                 |               |                 |
     |                 | llm request     |               |                 |
     |                 |----------------->               |                 |
     |                 |                 | llm request   |                 |
     |                 |                 |--------------------------------->
     |                 |                 | response      |                 |
     |                 |                 <---------------------------------|
     |                 | response        |               |                 |
     |                 <-----------------|               |                 |
     |                 |                 |               |                 |
     |                 | submission      |               |                 |
     |                 |----------------->               |                 |
     |                 |                 | submission    |                 |
     |                 |                 |--------------->                 |
     |                 |                 |               |--. run the game verifier
     |                 |                 |               |<-'              |
     |                 |                 | verdict       |                 |
     |                 |                 <---------------|                 |
     |                 | verdict         |               |                 |
     |                 <-----------------|               |                 |
     |                 |                 |               |                 |
     | turn over       |                 |               |                 |
     <-----------------|                 |               |                 |
     |--. start the next turn until the clock runs out                     |
     |<-'              |                 |               |                 |
     | start agent     |                 |               |                 |
     |----------------->                 |               |                 |
     |                 |                 |               |                 |
     |                 | ... as above ...|               |                 |
     |                 |                 |               |                 |
     | turn over       |                 |               |                 |
     <-----------------|                 |               |                 |
     |--. the clock ran out, so start the last call                        |
     |<-'              |                 |               |                 |
     | start agent     |                 |               |                 |
     |----------------->                 |               |                 |
     |                 |                 |               |                 |
     | turn over       |                 |               |                 |
     <-----------------|                 |               |                 |
     |--. collect the logs and the entries, complete runs/<run>/run.json   |
     |<-'              |                 |               |                 |
```

## Game definitions

Benchmarks are implemented as games. A game verifies what an agent left and ranks the entries it kept, and a multiplayer game fights two entries against each other or has agents attack them, which is what tournaments are made of.

### Verdicts, fights and metrics

The sidecar dumps each request to a JSON access log, which `ava` collects into `runs/<run>/proxy.access.log`.

Durations, byte counts and the requested host come from nginx variables. Token counts, the served model and the time to the first token are scanned out of the response body by njs while it streams past. The ratelimit and key budget headers of every answer are captured too, so the newest one is the account state, and the cost the gateway reports per answer is summed into the run metrics.

The verifier is `ava score`, running in the scoring container without network access: `--game` verifies the submission left in `submission/` with the named game and prints the verdict with the name of the entry file, which the receive hook keeps for a passing push. `--game` with `--fight <directory>` fights the entries under `first/` and `second/` of that directory over `--combats`, one unless given, and prints the tally, which is how a tournament plays a pairing: one container of the scorer image per fight, no network, the entries mounted read only. `--metrics` and `--attempts` aggregate the collected logs after the run into the run record.

