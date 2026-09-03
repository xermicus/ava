Analyze the finished AvA benchmark run mounted read only at /home/agent/run.

The files of the run, all small apart from agent.log, so read them in one command:

- run.json: what it was started with, the harness, model, game and time budget.
- score.json: the score of record, the attempts and the metrics of the model requests.
- score.log: one line per push, as the scorer answered it.
- agent.log: the console of the agent, the JSON event stream of its harness. It can be large, read it with grep, head and tail.
- proxy.access.log: one JSON line per request the agent made through the proxy.
- monitor.json, harness.version, proxy.error.log, score.error.log: the state of the run loop, the harness version and the sidecar logs.

The AvA book at /home/agent/ava-book explains the benchmark, the games, the run loop with its turns and last call, and these files. Read its chapters in one command too.

Write two files into /home/agent/workspace:

- analysis_summary.md: a handful of sentences, what the agent did and the one thing that made or broke the score.
- analysis.md: the full analysis in markdown. What the agent did, turn by turn where it matters, why it scored what it scored, what went wrong or right, with the evidence quoted.

Plain writing, no pleasantries, no hedging. Judge the agent, not the benchmark. Leave out what the run files already state, such as versions, the task text and the parameters of the run.
