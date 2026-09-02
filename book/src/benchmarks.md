# Benchmark Games

A game is a task folder under `games` and a scorer of the same name in the `ava-game` crate.

- Single player: the task has a fixed goal. `fib-golf` is one.
- Multi player: agents author puzzles for each other and solve what they receive.

## Playing a game

Run `ava agent -a <agent> -m <model> -g <name>`, where `-t` sets the seconds the agent is given and `-j` starts that many runs in parallel. Existing docker images are reused; `--force-build-images` rebuilds them.

`ava image` rebuilds every docker image ahead of runs; `-a <agent>`, `-p` (proxy) and `-s` (scorer) select single images.

`ava serve` starts the web interface on `localhost:2828`, `-p` picks another port. It starts and stops runs and shows them with their live state, next to the scoreboard, the games with their standings and the setup, rendered from the run artifacts on disk.

- `games/<name>` and `games/README.md` are mounted read only into the scorer container, which seeds them as the `master` commit of its repository before the hooks exist. The task description is `task.md`; the README holds the instructions shared by every game and a one line prompt points the harness at it. `AGENTS.md` and `CLAUDE.md` in the workspace are symlinks to it, the files every harness reads into context by itself when a session starts.
- Each harness loops itself over that prompt: claude through the ralph-wiggum plugin, pi through a staged extension, opencode through a staged server plugin, codex through a wrapper resuming its recorded session between turns. `ava` starts the sandbox once and only watches the clock and the done marker. The console goes to `runs/<run>/agent.log`; `ava` logs a status line every minute and warns when the agent stays silent for long or repeats one output line.
- The output tokens of a single turn are capped on every harness to the ceiling claude code uses for its own requests, so one turn cannot run out the whole clock.
- The agent starts in `/home/agent/workspace`, a tmpfs.
- The workspace is a clone of `http://git:8080/task.git`, checked out on the `task` branch. The agent submits by pushing that branch. A receive hook scores the push with `ava score --game <name>` and answers in the push output. `master` rejects every push.
- Every scored attempt is one line of `runs/<run>/score.log`. The best solving attempt is the submission of record.
- The run ends when the time is up, when the agent pushes a `release` tag, or when the harness exits. At the deadline the pi and opencode loops prompt once to commit and push. They deliver that prompt when a turn ends, so the grace holds while the agent keeps printing and the kill comes once it falls quiet or the run reaches 900 seconds past its limit.
- `ava score --metrics proxy.access.log --attempts score.log` aggregates the logs into `runs/<run>/score.json`: the attempts with the score of the run, the seconds to the best solving attempt breaking point ties, and the proxy metrics. The report written at the end of a run also names its parameters: the model, the harness and its version, the game and the time budget.

## Scoring a submission

- Points are the quantity the game optimizes, higher is better. `fib-golf` scores the halvings of the ELF byte size below a ceiling.
- Every game scores within 0 and 10000 points, which keeps runs of different games comparable.
- Attempts are recorded with the pure game grade and the arrival second, and the report only aggregates these facts. The final score weighing them against the run metrics, such as the time to the best attempt or the tokens spent, is derived at ranking time, so its knobs can change without re-running anything.
- The scorer executes the submission under a timeout and an output cap.
- A broken submission is an unsolved task, not an error. Only a broken harness fails a run.

## Adding a game

1. Write `games/<name>/task.md` and any files the agent needs.
2. Implement the `Game` trait in a new module of `ava-game`.
3. Add the implementation to the `GAMES` constant.

## Rating multi player matches

The playouts are not implemented yet, the ratings are.

A match is two halves. In each half one agent authors the puzzle and the other solves it, and the half is scored from three facts: the puzzle was valid, the generator solved it from the solver's view, the solver solved it.

- The generator takes the half by stumping the solver with a puzzle it solved itself.
- An invalid puzzle, or one the generator cannot solve from the solver's view, forfeits the half.
- Both sides solving is a draw.

`Scoring` turns played matches into a leaderboard. `bradley-terry` fits the whole match history, `elo` updates in match order.
