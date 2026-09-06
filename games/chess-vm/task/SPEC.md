# CVM: a chess-move scoring machine

You write one program in the CVM assembly language. The engine implements the
rules of chess. For each position it must move in, it takes the list of legal moves
and runs your program **once per move**; your program returns a single integer,
the move's score. The engine plays the highest-scoring move. Ties are broken by
a seeded pseudo-random generator.

A CVM program is therefore a move-scoring policy: how good this move is for the
side to move. You do not write the move loop, and you do not make or unmake
moves. The chess instructions read the current position and the move being
scored as implicit context.

## The machine

- 16 registers `r0` through `r15`, each a signed 32-bit integer. Arithmetic
  wraps on overflow. All comparisons are signed.
- A read-only `.data` section of up to 1024 words, addressed from 0, read with `LD`.
- Registers all start at 0 for every move.
- `SCORE ra` sets this move's score to the value in `ra`. `HALT` stops; the score
  from the last `SCORE` is the result (0 if the program never executed `SCORE`).

There is no writable memory, no stack, and no lookahead.

## Scoring model

Everything below is written from the point of view of **the side to move** (the
side your program is playing). "After the move" means the position that results
from playing the move being scored.

- Squares are `0..63`, numbered `rank * 8 + file`: a1 = 0, h1 = 7, a8 = 56, h8 = 63.
- Piece kinds: 1 pawn, 2 knight, 3 bishop, 4 rook, 5 queen, 6 king.
- Piece values in centipawns: pawn 100, knight 320, bishop 330, rook 500,
  queen 900. Kings are not counted in material.

## Instructions

`rd` is written; `ra` and `rb` are read; `imm` is a signed immediate (decimal,
`0x` hex, or a `.data` label). One instruction per line. Commas are optional and
mnemonics are case-insensitive. Each instruction costs the cycles in the table.

### Arithmetic and control

| Instruction | Effect | Cycles |
| --- | --- | --- |
| `LDI rd, imm` | `rd = imm` | 1 |
| `MOV rd, ra` | `rd = ra` | 1 |
| `ADD rd, ra, rb` | `rd = ra + rb` | 1 |
| `SUB rd, ra, rb` | `rd = ra - rb` | 1 |
| `MUL rd, ra, rb` | `rd = ra * rb` | 1 |
| `DIV rd, ra, rb` | `rd = ra / rb`, truncated toward zero; 0 if `rb == 0` | 1 |
| `MOD rd, ra, rb` | `rd = ra % rb`, sign of `ra`; 0 if `rb == 0` | 1 |
| `MIN rd, ra, rb` | `rd = min(ra, rb)` | 1 |
| `MAX rd, ra, rb` | `rd = max(ra, rb)` | 1 |
| `ADDI rd, ra, imm` | `rd = ra + imm` | 1 |
| `MULI rd, ra, imm` | `rd = ra * imm` | 1 |
| `NEG rd, ra` | `rd = -ra` | 1 |
| `LD rd, ra, imm` | `rd = data[ra + imm]` (the `.data` section) | 2 |
| `JMP label` | jump | 1 |
| `JZ ra, label` | jump if `ra == 0` | 1 |
| `JNZ ra, label` | jump if `ra != 0` | 1 |
| `JLT ra, rb, label` | jump if `ra < rb` | 1 |
| `JLE ra, rb, label` | jump if `ra <= rb` | 1 |
| `JGT ra, rb, label` | jump if `ra > rb` | 1 |
| `JGE ra, rb, label` | jump if `ra >= rb` | 1 |
| `SCORE ra` | set this move's score to `ra` | 1 |
| `HALT` | stop | 1 |

### Chess instructions

Each writes one register (some also take a square in `ra`). All read the implicit
(position, move) context.

| Instruction | Value written to `rd` | Cycles |
| --- | --- | --- |
| `MATERIAL rd` | our material minus the opponent's, after the move, centipawns | 32 |
| `GAIN rd` | value of the piece this move captures, 0 if none | 1 |
| `MOBILITY rd` | our pseudo-legal move count after the move | 200 |
| `OPPMOB rd` | the opponent's legal replies after the move | 200 |
| `GIVESCHECK rd` | 1 if the move gives check, else 0 | 1 |
| `ISCAPTURE rd` | 1 if the move captures, else 0 | 1 |
| `ISCASTLE rd` | 1 if the move castles, else 0 | 1 |
| `ISPROMO rd` | 1 if the move promotes, else 0 | 1 |
| `PROMO rd` | promotion piece kind (0, or 2/3/4/5), 0 if not a promotion | 1 |
| `MATEIN1 rd` | 1 if the move is checkmate, else 0 | 200 |
| `KINGDIST rd` | Chebyshev distance between the two kings after the move | 2 |
| `OWNKINGDIST rd` | summed distance of our pieces to our own king, after the move | 32 |
| `ENEMYKINGDIST rd` | summed distance of our pieces to the enemy king, after the move | 32 |
| `EXPOSED rd` | value of our most valuable piece the opponent attacks, after the move | 96 |
| `LIGHTCOUNT rd` | how many of our pieces stand on light squares, after the move | 32 |
| `SEE rd` | static exchange value of the move on its destination square, centipawns | 128 |
| `HANGS rd` | 1 if `SEE` is negative (the move loses the exchange), else 0 | 128 |
| `SAFE rd` | 1 if the destination has at least as many of our defenders as enemy attackers | 96 |
| `ATTACKERS rd, ra` | number of enemy pieces attacking square `ra`, after the move | 48 |
| `DEFENDERS rd, ra` | number of our pieces attacking (defending) square `ra`, after the move | 48 |
| `PIECEAT rd, ra` | piece at square `ra` after the move: `+kind` ours, `-kind` theirs, 0 empty | 2 |
| `MOVECOUNT rd, ra` | how many times the piece now on square `ra` has moved this game | 2 |
| `FROMSQ rd` | the square moved from, `0..63` | 1 |
| `TOSQ rd` | the square moved to, `0..63` | 1 |
| `TORANK rd` | destination rank from our side, 0 (our back rank) to 7 (promotion rank) | 1 |
| `MOVEDPIECE rd` | kind of the piece moved, `1..6` | 1 |
| `CENTER rd` | centrality of the destination, 0 (corner) to 6 (centre) | 1 |
| `PLY rd` | current ply number, 0 at the start of the game | 1 |

`MATERIAL`, `SEE` and similar instructions already account for the move:
`MATERIAL` after a capture includes the captured piece; `SEE` also accounts for
the opponent's recapture and any following exchange on that square.

## Assembly syntax

```
; a comment runs to the end of the line
label:                      ; a label on its own line
    LDI r1, 0x10            ; decimal, 0x hex and negative immediates
    LDI r2, table          ; a data label is its address
    JZ  r1, done
done:
    SCORE r1
    HALT

.data                       ; everything after this line is data
table:  .word 100, 320, 330, 500, 900
buf:    .zero 8             ; 8 words of zero
```

- Commas are optional; they read as whitespace.
- A code label is the index of the instruction after it. A data label is the
  address of the word after it in the `.data` section (addresses start at 0).
- `.data` must come after all the code.

## Limits and faults

| | |
| --- | --- |
| Instructions | 256 |
| Data words | 1024 |
| Cycles per move decision | 20,000,000 (across scoring every legal move) |

The assembler rejects a program over the code or data limit, with the line
number, before anything runs. There is no lookahead: the assembler rejects
`.search` in a submission.

A **fault** or a **budget overrun** while scoring a move ends that decision: the
engine plays move index 0 instead and counts the failure. Faults are: jumping outside the
code, running off the end without `HALT`, an `LD` outside the `.data` section,
and a square operand outside `0..63`. Dividing by zero is not a fault.

A program that faults or overruns anywhere in the verified games fails, whatever
its rating. The CI reports the first fault and its line number.

## Verification

Your bot plays 20 games, colours alternating, against each of 27 built-in
opponents, ranging from a mover that scores every move the same to a four ply
search. The score against them is converted into a rating on the ELO scale.

Every push is verified this way, and the CI reports the rating of your bot and
the ratings of the weakest and the strongest opponent. The seeds are fixed, so
the same program always gets the same rating.
