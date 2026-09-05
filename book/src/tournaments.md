# Tournaments

A tournament is a lobby of seats playing rounds of one game. Any game can be played: what a round does with the entries depends on the playout of the game, described under [the pairings](#the-pairings). Seating two of the same agent is allowed, and the standings rate agents, so their pairings against each other count for nothing.

## The lobby

`ava tournament -n <name> -g <game> -t <seconds> -c <combats>` opens a tournament, or the tournaments page of the web interface does. The game, the seconds every run is given and the combats every fight plays, 5 unless chosen otherwise, are fixed at that point, and so is the analyst: `--analyst harness/model[/thinking]` with `--analyst-seconds`, or the analyze checkbox of the form with its seconds field, has every run of a round analyzed the moment it is over, the runs of the seats and the runs of the attacks alike, in parallel with what the round still plays and under the same cap. A failed analysis fails nothing, the run page offers it again.

A seat holds an agent: a harness on a model at a thinking level. `-s harness/model` or `-s harness/model/thinking` seats one, repeatable, or the seat row on the tournament page does. Two seats may hold the same agent, which doubles its games a round; the standings rate agents, so the two seats are one entry there. The lobby is open until the first round is played and fixed from then on: seats neither join nor leave, since the rounds reference seats by their number and every round is played by the same lobby. A new agent gets a new tournament.

## A round

A round is every seat playing a run of the game, all at once, then every pairing of the entries they kept fighting. `ava tournament -n <name>` plays one, and the play round button on the tournament page does. `-j <runs>`, or the field beside the button, caps how many runs a phase has going at once; without it every run of a phase starts together, which for a played round is one per ordered pair of seats. A queued attack shows as queued in the cross table until its run starts. The runs are ordinary runs: they show up in the runs table like any other, naming the tournament, round and seat they play, and their pages are the run pages.

The round is written to the record the moment its runs are named, so the tournament page links the runs while they play. Once every run is over, the entry of record of each is picked: the entry ranking highest, the newest on ties, which for a pass or fail game is the last passing push. Then the pairings of the round robin are played, every pair of seats once. The record is written after every pairing, so a round that breaks off leaves what it had, and only a finished round counts for the standings.

## The pairings

- Automated playout, the r2wars games: the scorer fights the two entries over the combats the tournament fixed. Each fight is staged into the scorer image of the game with no network and the entries mounted read only, and its rounds are tallied from the view of the first seat. The console of the fights is `tournaments/<name>/round-<number>.log`. A seat that left no passing entry forfeits its pairings, which are recorded as one round lost with the reason. A pairing whose fight failed is recorded with no rounds and the reason, and counts for nothing.
- Single playout, fib-golf and sanity-check: the entries stand alone, so nothing is fought and nothing is recorded beyond the entries. The pairings are derived when the standings are shown, by comparing the points of the two entries of record: more points win the pairing, equal points draw it, as does a game ranking nothing, a missing entry forfeits it. A changed curve changes who won without touching the record.
- Played playout, the crackme games: one seat's entry is another seat's task. Every seat attacks the entry of every other seat in a run of the challenge game, all at once: the attacker's harness starts on the challenge task with the defender's entry seeded into its workspace, and the verifier checks its pushes against that entry, mounted into the scoring container. The pairing is recorded as one round to the attacker when a push passed and one round to the defender otherwise, with the run that played it, so the cross table links every attack. The pairings are every ordered pair of seats, since each seat attacks and defends, and the attacks on a seat that left no entry are forfeited to the attacker, or recorded with no rounds when the attacker left none either. A played round starts one run per ordered pair, six for three seats.

## The standings

The record holds tallies, or for a single playout the entries the tallies are compared from. Everything else is derived when the tournament page renders: the matches of the finished rounds between different agents, each scored for the first seat as the rounds won plus half the rounds drawn over the rounds played, and the two ratings over those matches. Elo walks the matches in the order they were fought and moves after every one. Bradley-Terry fits the whole history at once and orders the standings. Both are anchored at 1000. The standings list every agent's fights as won-drawn-lost, a fight with more rounds won than lost is won, and its score, the rounds won plus half the rounds drawn over the rounds played. The cross table of every round shows the tally of the row's seat against the column's seat, `forfeit` for a pairing one side did not field an entry for and `none` for one that saw no fight, with the reason behind a tooltip, the cell linking to the run that played the pairing when one did, and a total of the row's fights as won-drawn-lost. A line above it counts the seats that left an entry and what became of the pairings.

## Files

- `tournaments/<name>/tournament.json` is the record described in the [data model](data_model.md).
- `tournaments/<name>/round-<number>.log` is what the fights of the round printed.
- `tournaments/<name>/playing` holds the pid of the process playing a round while it does, which is how the web interface knows a round the command line plays is going on.
- The runs of the rounds are under `runs/`, like every run.
