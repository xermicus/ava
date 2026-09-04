# Running benchmarks

## Playing a game

Run `ava agent -a <agent> -m <model> -g <name>`, where `-t` sets the seconds the agent is given, the last call included, and `-j` starts that many runs in parallel. Existing docker images are reused; `--force-build-images` rebuilds them.

`ava image` rebuilds every docker image ahead of runs; `-a <agent>`, `-p` (proxy) and `-s` (scorer) select single images.

`ava serve` starts the web interface on `localhost:2828`, `-p` picks another port. It starts and stops runs and shows them with their live state, next to the scoreboard, the games with their standings, the tournaments with their lobbies and rounds, and the setup, rendered from the records on disk.

`ava --usage` asks every backend for its limits and prints them with the usage recorded over the runs on disk. The setup page shows the same.

- `games/<name>/task` and `games/README.md` are mounted read only into the scorer container, which seeds them as the `master` commit of its repository before the hooks exist. The task description is `task.md`; the README holds the instructions shared by every game and a one line prompt points the harness at it. `AGENTS.md` and `CLAUDE.md` in the workspace are symlinks to it, the files every harness reads into context by itself when a session starts.
- `ava` loops the harness over that prompt. A turn is one start of the harness: it answers, it exits, and that exit is the boundary the next turn starts on with `Loop iteration N.` ahead of the same prompt. Every harness resumes its recorded session by its own option, `--continue` for claude, pi and opencode and `codex exec resume` for codex, so the conversation accumulates across turns instead of restarting. The loop is one thing rather than one per harness, and the turn count is something the run records rather than something only the harness knows. `ava` starts the sandbox and only watches the clock and the done marker, and starts it a second time for the last call. The console goes to `runs/<run>/agent.log`, cut at 4 GiB; `ava` logs a status line every minute and warns when the agent stays silent for long or repeats one output line.
- The output tokens of a single turn are capped to the ceiling claude code uses for its own requests, 32000, or 64000 at the highest thinking levels. Codex has no per turn output knob, so the cap goes unenforced there. It bounds what a turn spends and not how long it takes: a model streaming at 20 tokens a second reaches the cap after 1600 seconds, so a single turn can and does outlast a phase.
- The agent starts in `/home/agent/workspace`, inside a home of its own described under [The agent home](#the-agent-home).
- The workspace is a clone of `http://git:8080/task.git`, checked out on the `task` branch. The agent submits by pushing that branch. A receive hook verifies the push with `ava score --game <name>` and answers in the push output with the verdict, passed or failed with the reason, and never with points. A tar posted to `http://score:8080` is verified the same way and not recorded. `master` rejects every push.
- Every graded push is one line of `runs/<run>/score.log`, and the entry file of every passing push is kept as `runs/<run>/entries/<seconds>/<entry>`. Which entry is the entry of record is decided when standings are shown.
- The run ends when the time is up, when the agent pushes a `release` tag, or when the harness exits. Running out of time is not the end of the run, it is the last call described below.

- `runs/<run>/run.json` is the record of the run, described in the [data model](data_model.md). It is written when the run starts with the harness and its version, the model, the game and its version and the time budget, and completed when the run is over with every attempt and the metrics aggregated from the proxy log.

## The agent home

`/home/agent` is a volume of the run's own, a tmpfs the local volume driver mounts, with a second one of the same kind nested at `/home/agent/workspace`. Three things follow from that.

The size is a limit the kernel enforces. `--storage-opt size` cannot do this job: under overlay2 it needs an xfs backing filesystem mounted with `pquota`, and under btrfs it refuses any size below the image and enforces nothing unless the host has qgroups enabled. A tmpfs needs nothing from the host.

Nothing the agent writes reaches the host disk, and removing the volume at teardown is what deletes it.

The home outlives the sandbox, which is what makes the last call possible.

It is also the portable way to get all three. The volume and its tmpfs are created by the docker daemon, so on macOS this happens inside its linux virtual machine and behaves exactly as it does on linux. Nothing here needs a host ramdisk, a loop device, a filesystem quota or root on the host.

The cost is that the home is memory. A run holds the copied home for its whole length, the base image with the harness and the toolchains of the game on top, so `-j` multiplies it and the ceiling of 4 GiB of home plus 4 GiB of workspace plus 2 GiB of scratch is per run.

The workspace is split off because the two sides fail differently. A tool call whose output never ends is what fills a filesystem here, and the harness writes that output into its own session store under the home. With one volume that took the workspace with it, and the last chance could not commit, so the run recorded nothing at all. With two, a full home costs the harness its session and leaves the submission untouched. The workspace is the side that has to keep working, since git writes to commit, so it is not sized to absorb anything beyond the task. For the same reason the git identity travels in the environment instead of `$HOME/.gitconfig`: writing that file is the first thing the bridge does, and on a full home it aborts before reaching git at all.

A single file is capped at half the smaller volume through `RLIMIT_FSIZE`, so one runaway write dies of `SIGXFSZ` rather than filling a tmpfs. It bounds one file and not a total, which is the shape of what has actually gone wrong.

Two details follow from overmounting a directory the image installs into. The volume is not populated from the image on its own, so `ava` copies the image home in before the agent starts, harness and toolchains included; the size has to cover that copy on top of what the harness writes. And a tmpfs is torn down once the last container using it exits, so a holder container keeps both mounted for the whole run.

Everything else the agent could write to is closed off. The root filesystem is read only, which holds against the passwordless `sudo` in the image because the container has no `CAP_SYS_ADMIN` to remount it. `/tmp` is a small tmpfs of its own and no restart carries it over.

## The last call

When the time is up the agent is stopped and started again on the home it left behind, prompted once to commit and push. It is one more turn of the loop, resuming the same session, and the only things that set it apart are the prompt and that no turn follows it.

The prompt states the bound as the final turn, not as seconds. Without a loop the start is one turn, so that is what holds, and the agent has nothing to time itself against. The 120 seconds are the timeout on that turn, not a budget the prompt asks it to spend, and the prompt also says that submitting is free, since every passing push keeps its entry and a failing one is only a failed attempt.

The prompt is an argument to that start, which is what makes it arrive at all. A file a harness polls cannot: anything watching between turns is never read by a turn that outlasts the wait.

Nothing has to be switched off for it either. The loop is `ava` deciding to start another turn, so the last call is `ava` deciding not to.

## Verifying and ranking a submission

- The verifier of the game runs on every push and records a fact: the push passed or failed, with the reason. It executes the submission under a timeout and an output cap, in the scoring container without network.
- A failing submission is a failed attempt, not an error. Only a broken harness fails a run.
- Ranking is derived from the entries when standings are shown. Every game ranks an entry within 0 and 10000 points from the file alone, which keeps runs of different games comparable, and how each game ranks is in [Games](games.md). The entry of record of a run is the entry ranking highest, the newest on ties.
- Since the ranking is derived, its knobs can change without re-running anything: a changed curve re-ranks every entry ever kept. The version of the game folder recorded on the run names the verifier the verdicts were reached under.
- Live runs show their pushes and whether one passed. Their points appear once the run is over, since the entries stay in the scoring container until then.

## Analyzing a run

`ava analyze -r <run> -a <agent> -m <model>` starts the agent once on `runs/<run>` and this book, both mounted read only, with the analysis prompt as its task. The summary of a few sentences and the full analysis in markdown it writes are assembled into `analysis.json` in the run directory, next to the analyst's console `analysis.log` and the access log of its proxy `analysis.access.log`; a failed analysis leaves its reason there instead. The run page shows the summary with the analysis folded behind it and offers the same behind a button. `-e` and `-t` set the thinking level and the seconds, 1200 by default. The start panel of the web interface can have a run analyzed the moment it is over, by claude on claude-sonnet-5 at medium thinking unless chosen otherwise, and the run shows as analyzing meanwhile.

## Playing tournaments

Games whose entries fight each other are played in [tournaments](tournaments.md): a lobby of seats, each holding an agent, plays rounds of runs followed by the fights of every pairing. The record holds the tallies of the fights and the standings are derived from it.
