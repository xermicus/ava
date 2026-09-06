# Chess: a move scoring policy

Write `bot.cvm`: a chess move scoring policy in the CVM assembly language, which
`SPEC.md` specifies in full.

The engine implements the rules of chess. For every position it has to move in,
it runs your program once for each legal move and plays the move your program
scores highest. You write the scoring, not the game loop, and you do not make or
unmake moves. Chess instructions read the position and the move being scored:
`MATERIAL`, `GAIN`, `SEE`, `MOBILITY`, `MATEIN1`, `EXPOSED` and the rest of the
table in `SPEC.md`.

The limits: 256 instructions, 1024 data words, 20 million cycles per decision,
and no lookahead. The assembler rejects `.search` in a submission.

The `bot.cvm` in this workspace scores every move the same, so it plays at
random. Replace it.

## How it is verified

Every push plays your bot 20 games against each of 27 built-in opponents,
ranging from that random mover to a four ply search. The score against them is
converted into a rating, and the result of the push reports the rating of your
bot and the ratings of the weakest and the strongest opponent. Beating the weak
half is easy; the searching opponents are the upper limit for a one ply policy.

A bot that faults or exhausts its cycle budget without producing a score fails,
whatever its rating, because the engine then plays the first legal move for it.
`SPEC.md` lists what faults. The CI reports the first fault and its line number.

The deliverable is `bot.cvm` in the root of the `task` branch.
