# Reports

A report is one document over the tournaments chosen for it: how every agent did against the tokens and the seconds it spent, over their finished rounds. The tournaments table on the tournaments page has a box beside every tournament, and the report button under it opens the report over the checked ones at `/report?tournament=<name>&tournament=<name>`. The page carries the styles and the fonts inside it and fetches nothing from the server, so saved from the browser it reads the same anywhere; the download link at its top hands it over as `report-<name>-<name>.html` straight away. A report of tournaments still playing is what their finished rounds hold and grows with them, so a report worth keeping is one saved once the tournaments are over.

The report is about models. Every chart line and every table row is one model pooled over every harness that drove it and every seat it held in the chosen tournaments, and the last section splits the models by harness. Only pairings between different agents, a harness on a model, count for the rounds and the score. The dollars are the tokens at the prices of the registry, so a run on a route without a price has none, and the ratios over dollars run over the runs that have one, with the count of the others behind the hover. The analyses of the runs are the analyst's tokens, not the agent's, and stay out.

## What it holds

Three tiles name the winners: the model with the highest score, the one that won a round for the fewest dollars and the one that won a round for the fewest tokens, each pooled over every harness that drove it.

Under the tiles the tournaments table, one row per model: its score over every chosen tournament, its score in each of them, blank where it had no round, and what a run that passed the verifier cost it, in dollars and in tokens through the backend.

Three charts follow, drawn like the charts of a tournament page and hovering the same way:

- Passes over the budget, one stepped line per model: the share of its runs that had a passing push by every share of the budget, so a line climbing early belongs to a model that passes fast and a line staying low to one that passes rarely. The budget is the share of the seconds a run was given, which is what lets tournaments of different lengths share the chart.
- Score against dollars and score against output tokens, one mark per model: the share of its rounds won against what one of its runs cost, in dollars and in thousands of output tokens. The marks towards the top left won cheaply.

One table over every model follows: the mean second of the scoring clock at which its first push passed and the mean tokens through the backend until then, over the runs that passed; the mean second at which its entry of record was pushed, the best entry of the run, and the mean tokens until then, over the runs that kept one; its score; and its cost per score, the dollars of its runs over the rounds it won. Blank cells mean no run got that far. The table arrives sorted by score and its headers sort it by any column, a blank cell last either way, the way the tables of the interface do.

The models and harnesses chart puts every model in a slot of its own, the best leftmost, and one mark per harness in it at the score that harness reached with the model, so the height of a slot's marks is the model and their spread is the harness. The line under the chart splits the variance of the agents' scores, every harness on every model, into the part between the means of the models, the part between the means of the harnesses, and what neither explains.

The harness efficiency section holds the same table and charts one row and one mark per harness. Chips over both sections choose the runs they cover: over the models every harness or one harness that drove more than one model, over the harnesses every model or one model that more than one harness drove, the best first. Only the claude harness drives the Anthropic models, so a harness pooled over every model it drove mostly measures its models, and a single model compares the harnesses on equal ground, as a single harness does the models. A model and a harness keep their colour on every chip.
