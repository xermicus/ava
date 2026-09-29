//! The report over chosen tournaments: how every agent did against the
//! tokens and the seconds it spent, and every chosen tournament as its page
//! shows it, as one document standing on its own, the styles, the fonts and
//! the images inside it and the pages of the agents and the runs behind their
//! links, so it reads the same saved as served.

use crate::{chart, views};
use ava_run::{docker, registry, runs, tournament, usage};

/// The directory the reports are written into unless another is named.
pub const DEFAULT_DIRECTORY: &str = "reports";

/// A part of a report: tournaments under a label of their own, the file
/// switching between its parts. A report in one piece is one part without a
/// label.
#[derive(Clone, Debug, Default)]
pub struct Part {
    pub label: String,
    pub names: Vec<String>,
}

/// The report command: the report over tournaments as a file.
#[derive(Debug)]
pub struct Report {
    /// The parts of the report, one piece over every tournament on disk when empty.
    pub parts: Vec<Part>,
    /// The directory the file is written into.
    pub directory: std::path::PathBuf,
}

impl Default for Report {
    fn default() -> Self {
        Self {
            parts: Vec::new(),
            directory: DEFAULT_DIRECTORY.into(),
        }
    }
}

/// Write the report of `command` into its directory as a site.
pub fn run(command: &Report) -> std::io::Result<i32> {
    let parts = if command.parts.is_empty() {
        vec![Part {
            label: String::new(),
            names: tournament::list()?
                .into_iter()
                .map(|record| record.name)
                .collect(),
        }]
    } else {
        command.parts.clone()
    };
    write_site(&parts, &command.directory)?;
    println!("{}", command.directory.join(SITE_INDEX).display());

    Ok(0)
}

/// The layout of the site.
const SITE_INDEX: &str = "index.html";
const RUNS_DIRECTORY: &str = "runs";
const AGENTS_DIRECTORY: &str = "agents";
const LOGS_DIRECTORY: &str = "logs";
const ASSETS_DIRECTORY: &str = "assets";
const FONTS_DIRECTORY: &str = "fonts";
const SPRITES_DIRECTORY: &str = "sprites";
const LOGOS_DIRECTORY: &str = "games";
const TAILWIND_FILE: &str = "tailwind.js";
const TABLE_SORT_FILE: &str = "table-sort.js";
const LOGO_SUFFIX: &str = ".png";
const LOG_SUFFIX: &str = ".log.gz";
const PARENT_PREFIX: &str = "../";
/// Where the interface serves what a page of the site carries itself.
const ASSET_ROOT: &str = "/assets/";
/// A tournament is a tab of the report, so its links lead there.
const TOURNAMENT_ADDRESS_PREFIX: &str = "/tournament/";
/// The logs are too large to publish as they stand.
const GZIP: [&str; 3] = ["gzip", "-9", "-c"];
const KILOBYTE: f64 = 1_000.0;
const MEGABYTE: f64 = 1_000_000.0;

const TITLE: &str = "Agent vs Agent";
/// The pages of the agents and the runs the file carries: each hidden until
/// a link makes it the target of the address, which hides the report itself.
const PAGES_CLASS: &str = "report-pages";
const FRONT_CLASS: &str = "report-front";
const PAGE_CLASS: &str = "report-page";
const PAGE_TRAIL_CLASSES: &str = "text-base font-semibold text-neutral-100 truncate mb-6";
/// The address of no page at all, leading back to the report.
const BACK_ADDRESS: &str = "#";
const RUN_ADDRESS_PREFIX: &str = "/run/";
const AGENT_ADDRESS_PREFIX: &str = "/agent/";
const RUN_ID_PREFIX: &str = "run-";
const AGENT_ID_PREFIX: &str = "agent-";
const HEAD_END: &str = "</head>";
/// The tag of the layout loading the styles from the server, replaced by
/// the styles themselves.
const TAILWIND_TAG: &str = "<script src=\"/assets/tailwind.js\"></script>";
/// Where the layout loads the fonts from, replaced by the fonts themselves.
const FONT_ADDRESS_PREFIX: &str = "/assets/fonts/";
const BODY_CLASSES: &str = "bg-neutral-950 text-neutral-200 font-sans text-sm antialiased";
const MAIN_CLASSES: &str = "max-w-7xl mx-auto px-6 py-10";
/// The masthead of the file, the title.
const HEADING_CLASSES: &str = "text-4xl font-semibold tracking-tight text-neutral-100";
/// The about tab: one of the characters walking beside what the report is,
/// over the games as their page shows them.
const ABOUT_CLASSES: &str = "p-4 flex items-start gap-6";
const ABOUT_TEXT_CLASSES: &str = "max-w-2xl text-base leading-relaxed text-neutral-400";
const ABOUT: &str = "Coding agents, each a harness on a model, play games against each other in tournaments. \
                     The report covers their finished rounds: how every model scored, and what models and harnesses spent in dollars, tokens and time.";
/// The tabs of the report, in their order.
const TAB_TITLES: [&str; 5] = [
    "quality",
    "model efficiency",
    "harness efficiency",
    "tournaments",
    "about",
];
/// The fields of the radio buttons behind the tabs of the report and the
/// tabs of its tournaments, under the prefix of their part, and behind the
/// switch between the parts.
const TAB_FIELD: &str = "tab";
const SUBTAB_FIELD: &str = "subtab";
const PART_FIELD: &str = "part";
/// The fields of the chips choosing the harnesses the model efficiency covers
/// and the models the harness efficiency covers, and the chips covering them
/// all.
const HARNESS_FIELD: &str = "model-harness";
const MODEL_FIELD: &str = "harness-model";
const EVERY_HARNESS: &str = "every harness";
const EVERY_MODEL: &str = "every model";
/// The tabs of the report as a row, the tournaments and the parts as chips
/// like the ones choosing the tournaments on the page.
const TAB_STYLE: TabStyle = TabStyle {
    row: TAB_ROW_CLASSES,
    tab: TAB_CLASSES,
};
const SUBTAB_STYLE: TabStyle = TabStyle {
    row: SUBTAB_ROW_CLASSES,
    tab: CHOICE_CLASSES,
};
const PART_STYLE: TabStyle = TabStyle {
    row: PART_ROW_CLASSES,
    tab: CHOICE_CLASSES,
};
const SUBTAB_ROW_CLASSES: &str = "flex flex-wrap items-center gap-3 mt-5 mb-8";
const PART_ROW_CLASSES: &str = "flex flex-wrap items-center gap-3 mt-6";
const TAB_ROW_CLASSES: &str = "flex flex-wrap gap-1.5 mt-8 mb-3 pb-2 border-b border-neutral-800";
const TAB_CLASSES: &str = "flex items-center gap-2 rounded-md px-3 py-1.5 text-sm text-neutral-400 cursor-pointer \
                           hover:text-neutral-100 hover:bg-neutral-800/60 transition-colors \
                           peer-checked:bg-neutral-800 peer-checked:text-neutral-100 \
                           peer-focus-visible:outline peer-focus-visible:outline-2 peer-focus-visible:outline-indigo-500";
const FILE_PREFIX: &str = "report";
const FILE_SUFFIX: &str = ".html";
const NAME_SEPARATOR: &str = "-";
const DOWNLOAD_LABEL: &str = "download";
const NO_TOURNAMENTS_NOTE: &str = "no tournament chosen, check some above";
/// The chips choosing the tournaments of the report, on the page of the
/// interface: a hidden box each, the chip lit while it is checked, and the
/// two buttons at the end of the row.
const CHOOSER_CLASSES: &str = "p-4 flex flex-wrap items-center gap-3";
const CHOICES_CLASSES: &str = "flex flex-wrap items-center gap-3 grow";
const ACTIONS_CLASSES: &str = "flex items-center gap-3 ml-auto";
const CHOICE_FIELD: &str = "choice";
const CHOICE_CLASSES: &str = "flex items-center gap-2 rounded-md border border-neutral-800 bg-neutral-950 \
                              px-2.5 h-9 text-sm text-neutral-400 cursor-pointer transition-colors \
                              hover:text-neutral-100 hover:border-neutral-700 \
                              peer-checked:border-indigo-500/70 peer-checked:bg-indigo-500/10 peer-checked:text-neutral-100 \
                              peer-focus-visible:outline peer-focus-visible:outline-2 peer-focus-visible:outline-indigo-500";
const GENERATE_LABEL: &str = "generate";
const DOWNLOAD_CLASSES: &str = "inline-flex items-center rounded-md border border-neutral-700 px-4 font-medium \
                                text-neutral-300 hover:bg-neutral-800 transition-colors";
const NO_ROUNDS_NOTE: &str = "no finished round in the chosen tournaments";
const NO_MODELS_NOTE: &str = "no run in a finished round";
const PERCENT: f64 = 100.0;
const THOUSAND: f64 = 1_000.0;
const MILLION: f64 = 1_000_000.0;
/// Where an agent without a score or a price sorts: after every other.
const UNSCORED: f64 = -1.0;
const UNPRICED: f64 = f64::INFINITY;
const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE64_PAD: char = '=';
const BASE64_BLOCK_BYTES: usize = 3;
const BASE64_BLOCK_CHARACTERS: usize = 4;
const BASE64_BITS: u32 = 6;

const MODEL_HEADER: &str = "*model|the model at the thinking level its harness was asked for";
const HARNESS_HEADER: &str = "*harness|the harness at the thinking level it was asked for";
/// The cells in front of the columns of a row: the model.
const MODEL_CELLS: usize = 1;
/// The hues a series is coloured from, hashed from the name of its model.
const HUES: u64 = 360;
/// The hosts of a run that are not a backend: its git remote and its scorer.
const GIT_HOST: &str = "git";
const SCORE_HOST: &str = "score";
/// The request a push to the git remote ends in.
const PUSH_PATH: &str = "/git-receive-pack";
/// The token counts a logged request carries, all of them through the backend.
const TOKEN_FIELDS: [&str; 4] = [
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
];
const NO_HARNESSES_NOTE: &str = "no model with a round against another agent";
/// The room a model gets on either side of its slot on the harness chart.
const SLOT_MARGIN: f64 = 0.5;
/// What a scatter is about: its title and the explanation behind it, the
/// measure along its horizontal axis and the labels of its quadrants, top
/// left, top right, bottom left, bottom right.
struct Scatter {
    title: &'static str,
    tooltip: &'static str,
    measure: &'static str,
    quadrants: [&'static str; 4],
}

const DOLLARS_SCATTER: Scatter = Scatter {
    title: "score against dollars",
    tooltip: "the share of the rounds won against the dollars one run cost, at the prices of the registry",
    measure: "dollars per run",
    quadrants: [
        "good, cheap",
        "good, expensive",
        "weak, cheap",
        "weak, expensive",
    ],
};
const OUTPUT_SCATTER: Scatter = Scatter {
    title: "score against output tokens",
    tooltip: "the share of the rounds won against the thousands of output tokens one run generated",
    measure: "thousand output tokens per run",
    quadrants: [
        "good, few tokens",
        "good, many tokens",
        "weak, few tokens",
        "weak, many tokens",
    ],
};
const TIME_SCATTER: Scatter = Scatter {
    title: "score against time to high score",
    tooltip: "the share of the rounds won against the mean second of the scoring clock at which the entry of record was pushed",
    measure: "mean time to high score",
    quadrants: ["good, fast", "good, slow", "weak, fast", "weak, slow"],
};
const TOKENS_SCATTER: Scatter = Scatter {
    title: "score against tokens to high score",
    tooltip: "the share of the rounds won against the mean million tokens through the backend until the entry of record was pushed",
    measure: "mean million tokens to high score",
    quadrants: [
        "good, few tokens",
        "good, many tokens",
        "weak, few tokens",
        "weak, many tokens",
    ],
};
/// The names the script keeps the chosen sorts of the tables under.
const MODELS_TABLE: &str = "report-models";
const TOURNAMENTS_TABLE: &str = "report-tournaments";
const HARNESSES_TABLE: &str = "report-harnesses";
const OVERALL_HEADER: &str = "#overall|the share of the rounds against other agents won over every chosen tournament, half for a draw";
const COST_PER_SUCCESS_HEADER: &str = "#avg spent per pass|the dollars of all its runs at the prices of the registry, divided by the runs a push of which passed the verifier";
const TOKENS_PER_SUCCESS_HEADER: &str = "#avg tokens per pass|the tokens through the backend over all its runs, divided by the runs a push of which passed the verifier";
/// The column of the tournaments table holding the overall score.
const OVERALL_COLUMN: usize = 1;
/// The columns of the table, after the model.
const COLUMNS: [Column; 6] = [
    Column::FirstPass,
    Column::FirstPassTokens,
    Column::HighScore,
    Column::HighScoreTokens,
    Column::Score,
    Column::CostPerScore,
];
/// One measure of a model, a column of the table.
#[derive(Clone, Copy)]
enum Column {
    FirstPass,
    FirstPassTokens,
    HighScore,
    HighScoreTokens,
    Score,
    CostPerScore,
}

impl Column {
    /// The header of the column, with its explanation behind the hover.
    fn header(self) -> &'static str {
        match self {
            Self::FirstPass => {
                "#mean time to first pass|the mean second of the scoring clock at which the first push passed, over the runs that passed"
            }
            Self::FirstPassTokens => {
                "#mean tokens to first pass|the mean tokens through the backend until that push, input, cache and output alike"
            }
            Self::HighScore => {
                "#mean time to high score|the mean second at which the entry of record was pushed, the best entry of the run, over the runs that kept one"
            }
            Self::HighScoreTokens => {
                "#mean tokens to high score|the mean tokens through the backend until that push"
            }
            Self::Score => {
                "#score|the share of the rounds against other agents won, half for a draw"
            }
            Self::CostPerScore => {
                "#mean cost per score|the dollars of its runs at the prices of the registry over the rounds it won, a draw counting half"
            }
        }
    }

    /// The cell of the column for `sum`.
    fn cell(self, sum: &Sum) -> String {
        match self {
            Self::FirstPass => seconds_label(mean(&sum.first_pass_seconds)),
            Self::FirstPassTokens => count_label(mean(&sum.first_pass_tokens)),
            Self::HighScore => seconds_label(mean(&sum.high_score_seconds)),
            Self::HighScoreTokens => count_label(mean(&sum.high_score_tokens)),
            Self::Score => sum
                .score()
                .map(|score| format!("{score:.2}"))
                .unwrap_or_default(),
            Self::CostPerScore => sum
                .dollars_per_round_won()
                .map(usage::money)
                .unwrap_or_default(),
        }
    }
}

/// One run of a seat in a finished round: what it spent and what came of it.
struct Played {
    /// The tournament, by its place among the chosen ones.
    tournament: usize,
    setup: ava_wire::Setup,
    limit_seconds: u64,
    cost: Option<f64>,
    metrics: Option<ava_wire::Metrics>,
    /// The second of the first push that passed, on the scoring clock, and
    /// the tokens through the backend until then.
    first_pass: Option<u64>,
    first_pass_tokens: Option<u64>,
    /// The second of the entry of record, the best entry of the run, and the
    /// tokens through the backend until then.
    high_score: Option<u64>,
    high_score_tokens: Option<u64>,
    /// The rounds the seat got against other agents in the round, on the run
    /// of the last turn.
    rounds: ava_wire::Tally,
}

/// Every run played in the finished rounds of `record`.
fn played_runs(
    tournament: usize,
    record: &ava_wire::Tournament,
    registry: &registry::Registry,
) -> std::io::Result<Vec<Played>> {
    let game = ava_game::find(&record.game);
    let last_turn = game.map_or(0, |game| game.turns().len() - 1);
    let labels: Vec<String> = record.seats.iter().map(|seat| seat.agent.label()).collect();
    let mut played = Vec::new();

    for round in &record.rounds {
        if round.finished_seconds.is_none() {
            continue;
        }
        let mut tallies = vec![ava_wire::Tally::default(); record.seats.len()];
        for pairing in tournament::pairings(record, round)? {
            if labels.get(pairing.first) == labels.get(pairing.second) {
                continue;
            }
            if let Some(tally) = tallies.get_mut(pairing.first) {
                add_tally(tally, pairing.tally);
            }
            if let Some(tally) = tallies.get_mut(pairing.second) {
                add_tally(tally, views::mirrored(&pairing.tally));
            }
        }

        for entry in &round.entries {
            let Some(setup) = record.seats.get(entry.seat) else {
                continue;
            };
            let directory = std::path::Path::new(docker::RUN_DIRECTORY).join(&entry.run);
            let Ok(run) = runs::read(&directory) else {
                continue;
            };
            let last = entry.turn == last_turn;
            let cost = run
                .metrics
                .as_ref()
                .and_then(|metrics| registry.cost(&run.setup(), metrics));
            let requests = requests(&directory);
            let first_pass = run
                .attempts
                .iter()
                .enumerate()
                .find(|(_, attempt)| attempt.verdict.passed);
            let high_score = entry.attempt.and_then(|seconds| {
                run.attempts
                    .iter()
                    .enumerate()
                    .find(|(_, attempt)| attempt.seconds == seconds)
            });
            played.push(Played {
                tournament,
                setup: setup.clone(),
                limit_seconds: run.limit_seconds,
                cost,
                first_pass: first_pass.map(|(_, attempt)| attempt.seconds),
                first_pass_tokens: first_pass.and_then(|(index, attempt)| {
                    tokens_until(&requests, &run, index, attempt.seconds)
                }),
                high_score: entry.attempt,
                high_score_tokens: high_score.and_then(|(index, attempt)| {
                    tokens_until(&requests, &run, index, attempt.seconds)
                }),
                rounds: if last {
                    tallies[entry.seat]
                } else {
                    ava_wire::Tally::default()
                },
                metrics: run.metrics,
            });
        }
    }

    Ok(played)
}

/// One request the proxy of a run logged: the second it was answered, the
/// tokens it moved, and whether it was a push to the git host, which is what
/// puts an attempt on the wall clock.
struct Request {
    seconds: u64,
    tokens: u64,
    push: bool,
}

/// The requests of the run in `directory` from its proxy log, in order,
/// none when the log is not there.
fn requests(directory: &std::path::Path) -> Vec<Request> {
    let Ok(logged) = std::fs::read_to_string(directory.join(docker::ACCESS_LOG)) else {
        return Vec::new();
    };
    let count = |line: &serde_json::Value, field: &str| {
        line.get(field)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default()
    };

    logged
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|line| {
            let seconds = usage::epoch_of(line.get("time")?.as_str()?)?;
            let host = line.get("host")?.as_str()?;
            let push = host == GIT_HOST
                && line.get("method")?.as_str()? == "POST"
                && line.get("uri")?.as_str()?.ends_with(PUSH_PATH);
            let tokens = if host == GIT_HOST || host == SCORE_HOST {
                0
            } else {
                TOKEN_FIELDS.iter().map(|field| count(&line, field)).sum()
            };
            Some(Request {
                seconds,
                tokens,
                push,
            })
        })
        .collect()
}

/// The tokens of `requests` up to the attempt at `index` with `seconds` on
/// the scoring clock: put on the wall clock by the push that made it when
/// every attempt has its push in the log, else by the start of the run.
fn tokens_until(
    requests: &[Request],
    run: &ava_wire::Run,
    index: usize,
    seconds: u64,
) -> Option<u64> {
    let pushes: Vec<u64> = requests
        .iter()
        .filter(|request| request.push)
        .map(|request| request.seconds)
        .collect();
    if requests.is_empty() {
        return None;
    }
    let wall = if pushes.len() == run.attempts.len() {
        pushes[index]
    } else {
        run.started_seconds + seconds
    };

    Some(
        requests
            .iter()
            .filter(|request| !request.push && request.seconds <= wall)
            .map(|request| request.tokens)
            .sum(),
    )
}

fn add_tally(tally: &mut ava_wire::Tally, view: ava_wire::Tally) {
    tally.won += view.won;
    tally.drawn += view.drawn;
    tally.lost += view.lost;
}

/// What a group of runs spent and got, summed.
#[derive(Default)]
struct Sum {
    runs: u64,
    /// The runs a push of which passed the verifier.
    passed: u64,
    rounds: ava_wire::Tally,
    /// The dollars of the runs with a price, and how many had one.
    dollars: f64,
    priced: u64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    /// The share of its budget every run that passed had spent at its first
    /// pass, the second of that pass and the tokens until it.
    first_pass_shares: Vec<f64>,
    first_pass_seconds: Vec<f64>,
    first_pass_tokens: Vec<f64>,
    /// The second every run that kept an entry pushed its entry of record at,
    /// and the tokens until it.
    high_score_seconds: Vec<f64>,
    high_score_tokens: Vec<f64>,
}

impl Sum {
    fn add(&mut self, played: &Played) {
        self.runs += 1;
        self.passed += u64::from(played.first_pass.is_some());
        add_tally(&mut self.rounds, played.rounds);
        if let Some(cost) = played.cost {
            self.dollars += cost;
            self.priced += 1;
        }
        if let Some(metrics) = &played.metrics {
            self.input_tokens += metrics.input_tokens;
            self.output_tokens += metrics.output_tokens;
            self.cache_read_tokens += metrics.cache_read_tokens;
            self.cache_write_tokens += metrics.cache_write_tokens;
        }
        if let Some(seconds) = played.first_pass {
            self.first_pass_shares
                .extend(budget_share(seconds, played.limit_seconds));
            self.first_pass_seconds.push(seconds as f64);
        }
        self.first_pass_tokens
            .extend(played.first_pass_tokens.map(|tokens| tokens as f64));
        self.high_score_seconds
            .extend(played.high_score.map(|seconds| seconds as f64));
        self.high_score_tokens
            .extend(played.high_score_tokens.map(|tokens| tokens as f64));
    }

    /// The rounds won, a draw counting half.
    fn rounds_won(&self) -> f64 {
        self.rounds.won as f64 + self.rounds.drawn as f64 / 2.0
    }

    fn score(&self) -> Option<f64> {
        self.rounds.score()
    }

    fn dollars_per_run(&self) -> Option<f64> {
        ratio(self.dollars, self.priced as f64).filter(|_| self.priced > 0)
    }

    fn dollars_per_round_won(&self) -> Option<f64> {
        ratio(self.dollars, self.rounds_won()).filter(|_| self.priced > 0)
    }

    /// The dollars over the runs that passed, nothing without a priced run.
    fn dollars_per_pass(&self) -> Option<f64> {
        ratio(self.dollars, self.passed as f64).filter(|_| self.priced > 0)
    }

    /// The tokens through the backend over the runs that passed.
    fn tokens_per_pass(&self) -> Option<f64> {
        ratio(self.tokens() as f64, self.passed as f64)
    }

    /// The tokens through the backend over the rounds it won, a draw counting half.
    fn tokens_per_round_won(&self) -> Option<f64> {
        ratio(self.tokens() as f64, self.rounds_won())
    }

    fn tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens + self.cache_read_tokens + self.cache_write_tokens
    }

    /// The output tokens one run generated, in thousands.
    fn thousand_output_per_run(&self) -> Option<f64> {
        ratio(self.output_tokens as f64 / THOUSAND, self.runs as f64)
    }
}

/// `numerator` over `denominator`, nothing over nothing.
fn ratio(numerator: f64, denominator: f64) -> Option<f64> {
    (denominator > 0.0).then(|| numerator / denominator)
}

/// The share of a budget of `limit` seconds spent at `seconds`, the whole
/// of it at the latest, nothing without a budget.
fn budget_share(seconds: u64, limit: u64) -> Option<f64> {
    ratio(seconds as f64, limit as f64).map(|share| share.min(1.0))
}

/// The runs sharing one key, summed, with the setup of the first of them.
struct Group {
    /// What the runs share: the name of the model, the harness or the agent.
    key: String,
    /// The colour of the group on a chart.
    hue: u64,
    setup: ava_wire::Setup,
    sum: Sum,
}

/// `played` grouped by `key`, the best score first and the cheapest run
/// among equals.
fn grouped<'a>(
    played: impl IntoIterator<Item = &'a Played>,
    key: impl Fn(&ava_wire::Setup) -> String,
) -> Vec<Group> {
    let mut groups: Vec<(String, Group)> = Vec::new();
    for run in played {
        let key = key(&run.setup);
        let group = match groups.iter_mut().find(|(known, _)| *known == key) {
            Some((_, group)) => group,
            None => {
                groups.push((
                    key.clone(),
                    Group {
                        hue: views::fnv1a(key.as_bytes()) % HUES,
                        key,
                        setup: run.setup.clone(),
                        sum: Sum::default(),
                    },
                ));
                &mut groups.last_mut().expect("a group was just pushed").1
            }
        };
        group.sum.add(run);
    }

    let mut groups: Vec<Group> = groups.into_iter().map(|(_, group)| group).collect();
    groups.sort_by(|left, right| {
        let score = |group: &Group| group.sum.score().unwrap_or(UNSCORED);
        let price = |group: &Group| group.sum.dollars_per_run().unwrap_or(UNPRICED);
        score(right)
            .total_cmp(&score(left))
            .then(price(left).total_cmp(&price(right)))
    });

    groups
}

/// The name of a setup along an axis of the report, its model or its harness.
type Name = fn(&ava_wire::Setup) -> &str;

fn model_name(setup: &ava_wire::Setup) -> &str {
    &setup.agent.model
}

fn harness_name(setup: &ava_wire::Setup) -> &str {
    &setup.agent.harness
}

/// `name` at the thinking level of `setup`, which is what models and
/// harnesses are told apart by: one model at two levels is two of them.
fn leveled(name: &str, setup: &ava_wire::Setup) -> String {
    match setup.thinking.as_deref() {
        Some(thinking) if !thinking.is_empty() => format!("{name} {thinking}"),
        _ => name.to_string(),
    }
}

fn agent_key(setup: &ava_wire::Setup) -> String {
    setup.label()
}

fn model_key(setup: &ava_wire::Setup) -> String {
    leveled(model_name(setup), setup)
}

fn harness_key(setup: &ava_wire::Setup) -> String {
    leveled(harness_name(setup), setup)
}

/// The hue of `harness` among `harnesses`: spread evenly over the wheel,
/// since a handful of names hashed can land next to each other.
fn harness_hue(harnesses: &[String], harness: &str) -> u64 {
    harnesses
        .iter()
        .position(|known| known == harness)
        .unwrap_or_default() as u64
        * HUES
        / harnesses.len().max(1) as u64
}

/// `played` grouped by harness, every group in the colour the harness has
/// among `harnesses` on every chart of the report.
fn grouped_by_harness<'a>(
    played: impl IntoIterator<Item = &'a Played>,
    harnesses: &[String],
) -> Vec<Group> {
    let mut groups = grouped(played, harness_key);
    for group in &mut groups {
        group.hue = harness_hue(harnesses, &group.key);
    }

    groups
}

/// The page of the interface: the boxes choosing the tournaments, the report
/// over the chosen ones under them.
pub(crate) fn page(names: &[String]) -> std::io::Result<String> {
    let mut body = chooser(names)?;
    body.push_str(&report(&records(names)?, "")?);

    Ok(views::page(&[views::REPORTS_SECTION], &body))
}

/// The report over `parts` as a file of its own, headed by the title, the
/// parts behind a switch under it when there are several, with the pages of
/// the agents seated in them and of every run on disk behind their links.
pub(crate) fn file(parts: &[Part]) -> std::io::Result<String> {
    let mut body = format!("<div class=\"{FRONT_CLASS}\">{}</div>", front_page(parts)?);
    let agents = views::agent_names()?;
    let mut runs = views::run_names()?;
    runs.sort();
    for agent in &agents {
        body.push_str(&subpage(
            AGENT_ID_PREFIX,
            agent,
            &views::agent_report(agent)?,
        ));
    }
    for run in &runs {
        body.push_str(&subpage(RUN_ID_PREFIX, run, &views::run_report(run)?));
    }
    for (prefix, id_prefix, names) in [
        (AGENT_ADDRESS_PREFIX, AGENT_ID_PREFIX, &agents),
        (RUN_ADDRESS_PREFIX, RUN_ID_PREFIX, &runs),
    ] {
        for name in names {
            let name = views::escape(name);
            body = pointed_inside(
                &body,
                &format!("{prefix}{name}"),
                &format!("{id_prefix}{name}"),
            );
        }
    }
    let body = format!(
        "<div class=\"{PAGES_CLASS}\"><style>{}</style>{body}</div>",
        page_rules()
    );

    Ok(document(&body, &views::games()?))
}

/// The report over `parts`, the masthead above it and the parts behind a
/// switch when there are several.
fn front_page(parts: &[Part]) -> std::io::Result<String> {
    let mut body = masthead();
    match parts {
        [part] if part.label.is_empty() => {
            body.push_str(&report(&records(&part.names)?, "")?);
        }
        _ => {
            let mut panes = Vec::new();
            for (index, part) in parts.iter().enumerate() {
                panes.push((
                    views::escape(&part.label),
                    report(&records(&part.names)?, &format!("{PART_FIELD}{index}-"))?,
                ));
            }
            body.push_str(&tabs(PART_FIELD, &PART_STYLE, &panes));
        }
    }

    Ok(body)
}

/// Write the report over `parts` into `directory` as a site: a page per run
/// and per agent of their own, so a visitor loads the report alone and takes
/// the whole log of a run as a download beside its page.
fn write_site(parts: &[Part], directory: &std::path::Path) -> std::io::Result<()> {
    let front = front_page(parts)?;
    let agents = views::agent_names()?;
    let mut runs = views::run_names()?;
    runs.sort();
    let games = views::games()?;

    write_assets(directory, &games)?;
    let logs = directory.join(LOGS_DIRECTORY);
    for path in [
        &logs,
        &directory.join(RUNS_DIRECTORY),
        &directory.join(AGENTS_DIRECTORY),
    ] {
        std::fs::create_dir_all(path)?;
    }

    let page = |body: &str, prefix: &str| {
        site_document(&linked(body, prefix, &runs, &agents), prefix, &games)
    };

    std::fs::write(directory.join(SITE_INDEX), page(&front, ""))?;
    for agent in &agents {
        std::fs::write(
            directory
                .join(AGENTS_DIRECTORY)
                .join(format!("{agent}{FILE_SUFFIX}")),
            page(&views::agent_report(agent)?, PARENT_PREFIX),
        )?;
    }
    for run in &runs {
        let mut body = views::run_report(run)?;
        if let Some(bytes) = write_log(run, &logs)? {
            body.push_str(&log_link(run, bytes));
        }
        std::fs::write(
            directory
                .join(RUNS_DIRECTORY)
                .join(format!("{run}{FILE_SUFFIX}")),
            page(&body, PARENT_PREFIX),
        )?;
    }

    Ok(())
}

/// What every page of the site shares, written once.
fn write_assets(directory: &std::path::Path, games: &[String]) -> std::io::Result<()> {
    let assets = directory.join(ASSETS_DIRECTORY);
    let fonts = assets.join(FONTS_DIRECTORY);
    let sprites = assets.join(SPRITES_DIRECTORY);
    let logos = assets.join(LOGOS_DIRECTORY);
    for path in [&assets, &fonts, &sprites, &logos] {
        std::fs::create_dir_all(path)?;
    }

    std::fs::write(assets.join(TAILWIND_FILE), crate::serve::TAILWIND)?;
    std::fs::write(assets.join(TABLE_SORT_FILE), crate::serve::TABLE_SORT)?;
    for (name, bytes) in crate::serve::FONTS {
        std::fs::write(fonts.join(name), bytes)?;
    }
    for (name, bytes) in crate::serve::SPRITES {
        std::fs::write(sprites.join(name), bytes)?;
    }
    for game in games {
        if let Some((logo, _)) = views::game_logo(game) {
            std::fs::write(logos.join(format!("{game}{LOGO_SUFFIX}")), logo)?;
        }
    }

    Ok(())
}

/// The document around `body`, `prefix` leading to the root of the site.
fn site_document(body: &str, prefix: &str, games: &[String]) -> String {
    let (head, _) = views::LAYOUT_TEMPLATE
        .split_once(HEAD_END)
        .expect("the layout has a head");
    let root = format!("{prefix}{ASSETS_DIRECTORY}/");
    let head = head
        .replace(views::TITLE_PLACEHOLDER, TITLE)
        .replace(ASSET_ROOT, &root);
    let mut body = body.replace(ASSET_ROOT, &root);
    for game in games {
        body = body.replace(
            &views::logo_address(game),
            &format!("{root}{LOGOS_DIRECTORY}/{game}{LOGO_SUFFIX}"),
        );
    }

    format!(
        "{head}{HEAD_END}<body class=\"{BODY_CLASSES}\"><main class=\"{MAIN_CLASSES}\">{body}</main>\
         <script src=\"{root}{TABLE_SORT_FILE}\"></script></body></html>"
    )
}

/// `body` with every link to one of `runs` or `agents` pointing at its page.
fn linked(body: &str, prefix: &str, runs: &[String], agents: &[String]) -> String {
    let mut body = body.to_string();
    for (address, directory, names) in [
        (RUN_ADDRESS_PREFIX, RUNS_DIRECTORY, runs),
        (AGENT_ADDRESS_PREFIX, AGENTS_DIRECTORY, agents),
    ] {
        for name in names {
            let name = views::escape(name);
            body = body.replace(
                &format!("href=\"{address}{name}\""),
                &format!("href=\"{prefix}{directory}/{name}{FILE_SUFFIX}\""),
            );
        }
    }

    pointed_at_report(&body, prefix)
}

/// `body` with every link to a tournament pointing at the report.
fn pointed_at_report(body: &str, prefix: &str) -> String {
    let mut pointed = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(at) = rest.find(TOURNAMENT_ADDRESS_PREFIX) {
        pointed.push_str(&rest[..at]);
        pointed.push_str(&format!("{prefix}{SITE_INDEX}"));
        rest = &rest[at..];
        rest = &rest[rest.find('"').unwrap_or(rest.len())..];
    }
    pointed.push_str(rest);

    pointed
}

/// The log of `run` packed into `directory` and how large it came out.
fn write_log(run: &str, directory: &std::path::Path) -> std::io::Result<Option<u64>> {
    let source = std::path::Path::new(docker::RUN_DIRECTORY)
        .join(run)
        .join(docker::AGENT_LOG);
    if !source.is_file() {
        return Ok(None);
    }

    let path = directory.join(format!("{run}{LOG_SUFFIX}"));
    let [program, arguments @ ..] = GZIP;
    let status = std::process::Command::new(program)
        .args(arguments)
        .arg(&source)
        .stdout(std::fs::File::create(&path)?)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "{program}: {status} on {run}"
        )));
    }

    Ok(Some(path.metadata()?.len()))
}

/// The link handing the whole log of `run` over.
fn log_link(run: &str, bytes: u64) -> String {
    format!(
        "<p class=\"{}\">console</p>\
         <p><a class=\"{}\" href=\"{PARENT_PREFIX}{LOGS_DIRECTORY}/{}{LOG_SUFFIX}\">the whole log</a> \
         <span class=\"{}\">{}</span></p>",
        views::TITLE_CLASSES,
        views::LINK_CLASSES,
        views::escape(run),
        views::MUTED_CLASSES,
        size_label(bytes)
    )
}

/// `bytes` as a size to read.
fn size_label(bytes: u64) -> String {
    match bytes as f64 {
        bytes if bytes >= MEGABYTE => format!("{:.1} MB", bytes / MEGABYTE),
        bytes => format!("{:.0} kB", bytes / KILOBYTE),
    }
}

/// The records of the tournaments `names`.
fn records(names: &[String]) -> std::io::Result<Vec<ava_wire::Tournament>> {
    names.iter().map(|name| tournament::load(name)).collect()
}

/// The page of `name` as `body`, under the id of `prefix` and the name,
/// headed by the trail back to the report.
fn subpage(prefix: &str, name: &str, body: &str) -> String {
    format!(
        "<section id=\"{prefix}{}\" class=\"{PAGE_CLASS}\"><h1 class=\"{PAGE_TRAIL_CLASSES}\">{}</h1>{body}</section>",
        views::escape(name),
        views::steps(&[(TITLE, BACK_ADDRESS), (name, "")])
    )
}

/// The rules showing the page the address targets in place of the report.
fn page_rules() -> String {
    format!(
        ".{PAGE_CLASS}{{display:none}}.{PAGE_CLASS}:target{{display:block}}\
         .{PAGES_CLASS}:has(.{PAGE_CLASS}:target)>.{FRONT_CLASS}{{display:none}}"
    )
}

/// `body` with every link to `address` pointing at the page `id` inside the file.
fn pointed_inside(body: &str, address: &str, id: &str) -> String {
    body.replace(&format!("href=\"{address}\""), &format!("href=\"#{id}\""))
}

/// The chips choosing the tournaments of the report, every tournament on
/// disk with the chosen ones lit, the button generating the report over them
/// and, once there is one, the button handing it over as a file.
fn chooser(names: &[String]) -> std::io::Result<String> {
    let choices: String = tournament::list()?
        .iter()
        .enumerate()
        .map(|(index, record)| {
            format!(
                "<span><input type=\"checkbox\" id=\"{CHOICE_FIELD}-{index}\" name=\"{}\" value=\"{}\" class=\"peer sr-only\"{}>\
                 <label for=\"{CHOICE_FIELD}-{index}\" class=\"{CHOICE_CLASSES}\">{}<span class=\"{}\">{}</span></label></span>",
                crate::serve::TOURNAMENT_FIELD,
                views::escape(&record.name),
                if names.contains(&record.name) {
                    " checked"
                } else {
                    ""
                },
                views::logo_cell(&record.game),
                views::MONO_CLASSES,
                views::escape(&record.name)
            )
        })
        .collect();
    let download = if names.is_empty() {
        String::new()
    } else {
        format!(
            "<a class=\"{DOWNLOAD_CLASSES} {}\" href=\"/report?{}&{}=on\">{DOWNLOAD_LABEL}</a>",
            views::CONTROL_HEIGHT,
            query(names),
            crate::serve::DOWNLOAD_FIELD
        )
    };

    Ok(format!(
        "<p class=\"{}\">tournaments</p>\
         <form method=\"get\" action=\"/reports\" class=\"{} {CHOOSER_CLASSES}\"><div class=\"{CHOICES_CLASSES}\">{choices}</div>\
         <div class=\"{ACTIONS_CLASSES}\">{download}<button class=\"{} {}\">{GENERATE_LABEL}</button></div></form>",
        views::FIRST_TITLE_CLASSES,
        views::CARD_CLASSES,
        views::BUTTON_CLASSES,
        views::CONTROL_HEIGHT
    ))
}

/// The query naming every tournament of `names`.
fn query(names: &[String]) -> String {
    names
        .iter()
        .map(|name| {
            format!(
                "{}={}",
                crate::serve::TOURNAMENT_FIELD,
                crate::serve::urlencode(name)
            )
        })
        .collect::<Vec<String>>()
        .join("&")
}

/// The report over `records`, or the note saying why there is none, its tabs
/// on fields under `prefix`, so the parts of a file switch on their own.
fn report(records: &[ava_wire::Tournament], prefix: &str) -> std::io::Result<String> {
    let registry = registry::load()?;
    if records.is_empty() {
        return Ok(note(NO_TOURNAMENTS_NOTE));
    }

    let mut played = Vec::new();
    for (index, record) in records.iter().enumerate() {
        played.extend(played_runs(index, record, &registry)?);
    }
    if played.is_empty() {
        return Ok(note(NO_ROUNDS_NOTE));
    }
    let mut body = String::new();

    let by_model = grouped(&played, model_key);
    let by_agent = grouped(&played, agent_key);
    let mut tournaments = tournaments_table(records, &played);
    tournaments.push_str(&tournaments_chart(records, &by_model, &played));
    tournaments.push_str(&harness_chart(&by_agent));
    let mut harnesses: Vec<String> = played.iter().map(|run| harness_key(&run.setup)).collect();
    harnesses.sort();
    harnesses.dedup();
    let by_harness = grouped_by_harness(&played, &harnesses);
    let efficiency = chip_panes(
        &played,
        by_harness
            .iter()
            .map(|group| harness_name(&group.setup))
            .collect(),
        harness_name,
        model_name,
        &format!("{prefix}{HARNESS_FIELD}"),
        EVERY_HARNESS,
        &|runs| model_section(&grouped(runs.iter().copied(), model_key)),
    );
    let harnesses = chip_panes(
        &played,
        by_model
            .iter()
            .map(|group| model_name(&group.setup))
            .collect(),
        model_name,
        harness_name,
        &format!("{prefix}{MODEL_FIELD}"),
        EVERY_MODEL,
        &|runs| harness_section(grouped_by_harness(runs.iter().copied(), &harnesses)),
    );
    let mut subpanes = Vec::new();
    for record in records {
        subpanes.push((
            format!(
                "{}<span class=\"{}\">{}</span>",
                views::logo_cell(&record.game),
                views::MONO_CLASSES,
                views::escape(&record.name)
            ),
            views::tournament_report(&record.name)?,
        ));
    }
    let panes: Vec<(String, String)> = TAB_TITLES
        .into_iter()
        .map(views::escape)
        .zip([
            tournaments,
            efficiency,
            harnesses,
            tabs(&format!("{prefix}{SUBTAB_FIELD}"), &SUBTAB_STYLE, &subpanes),
            about()?,
        ])
        .collect();
    body.push_str(&tabs(&format!("{prefix}{TAB_FIELD}"), &TAB_STYLE, &panes));

    Ok(body)
}

/// `section` over every run of `played`, then over the runs of each value of
/// `name` that more than one value of `other` shares, in the order of `order`,
/// behind chips on `field`, so one model can compare the harnesses that drove
/// it and one harness the models it drove.
fn chip_panes<'a>(
    played: &'a [Played],
    order: Vec<&'a str>,
    name: Name,
    other: Name,
    field: &str,
    every: &str,
    section: &dyn Fn(&[&'a Played]) -> String,
) -> String {
    let mut shared: Vec<&str> = Vec::new();
    for value in order {
        if shared.contains(&value) {
            continue;
        }
        let mut others: Vec<&str> = played
            .iter()
            .filter(|run| name(&run.setup) == value)
            .map(|run| other(&run.setup))
            .collect();
        others.sort_unstable();
        others.dedup();
        if others.len() > 1 {
            shared.push(value);
        }
    }

    let every_run: Vec<&Played> = played.iter().collect();
    let mut panes = vec![(views::escape(every), section(&every_run))];
    for value in shared {
        let runs: Vec<&Played> = played
            .iter()
            .filter(|run| name(&run.setup) == value)
            .collect();
        panes.push((
            format!(
                "<span class=\"{}\">{}</span>",
                views::MONO_CLASSES,
                views::escape(value)
            ),
            section(&runs),
        ));
    }

    tabs(field, &SUBTAB_STYLE, &panes)
}

/// The table and the charts of the model efficiency over `by_model`.
fn model_section(by_model: &[Group]) -> String {
    let mut efficiency = group_table(by_model, MODELS_TABLE, MODEL_HEADER, model_name, &COLUMNS);
    efficiency.push_str(&format!(
        "<div class=\"{}\">{}{}</div>",
        views::CHARTS_GRID_CLASSES,
        efficiency_chart(
            by_model,
            "cost efficiency",
            "the dollars of every model's runs at the prices of the registry over the rounds it won, a draw counting half, the cheapest leftmost",
            "dollars per round won",
            Sum::dollars_per_round_won,
            |dollars| format!("{} per round won", usage::money(dollars)),
        ),
        efficiency_chart(
            by_model,
            "token efficiency",
            "the tokens through the backend of every model's runs over the rounds it won, a draw counting half, in thousands, the leanest leftmost",
            "thousand tokens per round won",
            |sum| sum.tokens_per_round_won().map(|tokens| tokens / THOUSAND),
            |thousands| format!("{thousands:.0}k tokens per round won"),
        ),
    ));
    efficiency.push_str(&format!(
        "<div class=\"{}\">{}{}</div>",
        views::CHARTS_GRID_CLASSES,
        scatter(
            by_model,
            &DOLLARS_SCATTER,
            chart::Axis::values,
            |sum| sum.dollars_per_run(),
            |dollars| format!("{} per run", usage::money(dollars)),
        ),
        scatter(
            by_model,
            &OUTPUT_SCATTER,
            chart::Axis::values,
            Sum::thousand_output_per_run,
            |thousands| format!("{thousands:.0}k output tokens per run"),
        ),
    ));
    efficiency.push_str(&format!(
        "<div class=\"{}\">{}{}</div>",
        views::CHARTS_GRID_CLASSES,
        scatter(
            by_model,
            &TIME_SCATTER,
            |top| chart::Axis::seconds(top as u64),
            |sum| mean(&sum.high_score_seconds),
            |seconds| format!("high score after {}", usage::span(seconds as u64)),
        ),
        scatter(
            by_model,
            &TOKENS_SCATTER,
            chart::Axis::values,
            |sum| mean(&sum.high_score_tokens).map(|tokens| tokens / MILLION),
            |millions| format!("{millions:.1}M tokens to the high score"),
        ),
    ));
    efficiency.push_str(&pass_curve(by_model));

    efficiency
}

/// The table and the charts of the harness efficiency over `by_harness`.
fn harness_section(by_harness: Vec<Group>) -> String {
    let mut harnesses = group_table(
        &by_harness,
        HARNESSES_TABLE,
        HARNESS_HEADER,
        harness_name,
        &COLUMNS,
    );
    harnesses.push_str(&format!(
        "<div class=\"{}\">{}{}</div>",
        views::CHARTS_GRID_CLASSES,
        efficiency_chart(
            &by_harness,
            "cost efficiency",
            "the dollars of every harness's runs at the prices of the registry over the rounds it won, a draw counting half, over every model it drove, the cheapest leftmost",
            "dollars per round won",
            Sum::dollars_per_round_won,
            |dollars| format!("{} per round won", usage::money(dollars)),
        ),
        efficiency_chart(
            &by_harness,
            "token efficiency",
            "the tokens through the backend of every harness's runs over the rounds it won, a draw counting half, in thousands, over every model it drove, the leanest leftmost",
            "thousand tokens per round won",
            |sum| sum.tokens_per_round_won().map(|tokens| tokens / THOUSAND),
            |thousands| format!("{thousands:.0}k tokens per round won"),
        ),
    ));
    harnesses.push_str(&format!(
        "<div class=\"{}\">{}{}</div>",
        views::CHARTS_GRID_CLASSES,
        scatter(
            &by_harness,
            &DOLLARS_SCATTER,
            chart::Axis::values,
            |sum| sum.dollars_per_run(),
            |dollars| format!("{} per run", usage::money(dollars)),
        ),
        scatter(
            &by_harness,
            &OUTPUT_SCATTER,
            chart::Axis::values,
            Sum::thousand_output_per_run,
            |thousands| format!("{thousands:.0}k output tokens per run"),
        ),
    ));
    harnesses.push_str(&format!(
        "<div class=\"{}\">{}{}</div>",
        views::CHARTS_GRID_CLASSES,
        scatter(
            &by_harness,
            &TIME_SCATTER,
            |top| chart::Axis::seconds(top as u64),
            |sum| mean(&sum.high_score_seconds),
            |seconds| format!("high score after {}", usage::span(seconds as u64)),
        ),
        scatter(
            &by_harness,
            &TOKENS_SCATTER,
            chart::Axis::values,
            |sum| mean(&sum.high_score_tokens).map(|tokens| tokens / MILLION),
            |millions| format!("{millions:.1}M tokens to the high score"),
        ),
    ));
    harnesses.push_str(&pass_curve(&by_harness));

    harnesses
}

/// The name of the report over `parts` as a file: after the labels of the
/// parts, after the tournaments of a part without one.
pub(crate) fn file_name(parts: &[Part]) -> String {
    let mut name = FILE_PREFIX.to_string();
    for part in parts {
        let pieces: Vec<&str> = if part.label.is_empty() {
            part.names.iter().map(String::as_str).collect()
        } else {
            vec![part.label.as_str()]
        };
        for piece in pieces {
            name.push_str(NAME_SEPARATOR);
            name.push_str(piece);
        }
    }
    name.push_str(FILE_SUFFIX);

    name
}

/// The front of the file, its title.
fn masthead() -> String {
    format!("<h1 class=\"{HEADING_CLASSES}\">{TITLE}</h1>")
}

/// What the report is, beside one of the characters walking, and the games
/// under it.
fn about() -> std::io::Result<String> {
    Ok(format!(
        "<div class=\"{} {ABOUT_CLASSES}\">{}<p class=\"{ABOUT_TEXT_CLASSES}\">{ABOUT}</p></div>\
         <p class=\"{}\">games</p>{}",
        views::CARD_CLASSES,
        views::sprite(true),
        views::TITLE_CLASSES,
        views::games_report()?
    ))
}

fn note(text: &str) -> String {
    format!(
        "<p class=\"{} mt-8\">{}</p>",
        views::NOTE_CLASSES,
        views::escape(text)
    )
}

/// The score of every model under every harness that drove it: one slot per
/// model, one mark per harness in it, and under the chart how the variance of
/// the agents' scores splits between models and harnesses.
fn harness_chart(agents: &[Group]) -> String {
    let scored: Vec<&Group> = agents
        .iter()
        .filter(|group| group.sum.score().is_some())
        .collect();
    let mut harnesses: Vec<String> = scored
        .iter()
        .map(|group| harness_key(&group.setup))
        .collect();
    harnesses.sort();
    harnesses.dedup();
    let mut models: Vec<(f64, String)> = Vec::new();
    for group in &scored {
        let model = model_key(&group.setup);
        if models.iter().any(|(_, known)| *known == model) {
            continue;
        }
        let mut rounds = ava_wire::Tally::default();
        for peer in scored.iter().filter(|peer| model_key(&peer.setup) == model) {
            add_tally(&mut rounds, peer.sum.rounds);
        }
        models.push((rounds.score().unwrap_or_default(), model));
    }
    // The best model stands leftmost.
    models.sort_by(|left, right| right.0.total_cmp(&left.0));
    let slot = |model: &str| models.iter().position(|(_, known)| known == model);

    let series: Vec<chart::Series> = harnesses
        .iter()
        .map(|harness| chart::Series {
            label: harness.clone(),
            hover: harness.clone(),
            hue: harness_hue(&harnesses, harness),
            face: String::new(),
            points: scored
                .iter()
                .filter(|group| harness_key(&group.setup) == *harness)
                .filter_map(|group| {
                    let score = group.sum.score()?;
                    Some(chart::Point {
                        x: slot(&model_key(&group.setup))? as f64,
                        y: score * PERCENT,
                        hover: format!("{}, score {score:.2}", group.setup.label()),
                    })
                })
                .collect(),
        })
        .collect();
    let horizontal = chart::Axis {
        min: -SLOT_MARGIN,
        max: models.len() as f64 - 1.0 + SLOT_MARGIN,
        ticks: models
            .iter()
            .enumerate()
            .map(|(index, (_, model))| (index as f64, model.clone()))
            .collect(),
        title: String::new(),
        icons: Vec::new(),
    };

    let model = |group: &Group| model_key(&group.setup);
    let harness = |group: &Group| harness_key(&group.setup);
    let (by_model, by_harness, rest) = variance_shares(&scored, &model, &harness);

    format!(
        "{}<p class=\"{} mt-3\">{}</p>",
        views::chart_panel(
            "model to harness variance",
            "the score of every model under every harness that drove it, one mark per harness in the slot of the model, the best model leftmost",
            &chart::lines(
                &series,
                &horizontal,
                &chart::Axis::percent().titled("share of the rounds won"),
                chart::Shape::Scatter,
                None,
                chart::WIDE_WIDTH,
                NO_HARNESSES_NOTE,
            ),
        ),
        views::NOTE_CLASSES,
        views::explained(
            &format!(
                "variance of the agents' scores: models {}, harnesses {}, the rest {}",
                percent_label(by_model),
                percent_label(by_harness),
                percent_label(rest)
            ),
            "how far the scores of the agents, every harness on every model, spread around their mean, split into the part between the means of the models, the part between the means of the harnesses, and what neither explains"
        )
    )
}

/// How the variance of the scores of `scored` splits: the share between the
/// means of `factor`, the share between the means of `other`, and the rest,
/// none of them without two agents to spread.
fn variance_shares(
    scored: &[&Group],
    factor: &dyn Fn(&Group) -> String,
    other: &dyn Fn(&Group) -> String,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    let scores: Vec<f64> = scored
        .iter()
        .filter_map(|group| group.sum.score())
        .collect();
    if scores.len() < 2 {
        return (None, None, None);
    }
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let total: f64 = scores.iter().map(|score| (score - mean).powi(2)).sum();
    if total <= 0.0 {
        return (None, None, None);
    }
    let between = |key: &dyn Fn(&Group) -> String| -> f64 {
        scored
            .iter()
            .map(|group| {
                let peers: Vec<f64> = scored
                    .iter()
                    .filter(|peer| key(peer) == key(group))
                    .filter_map(|peer| peer.sum.score())
                    .collect();
                let peer_mean = peers.iter().sum::<f64>() / peers.len() as f64;
                (peer_mean - mean).powi(2)
            })
            .sum::<f64>()
            / total
    };
    let by_factor = between(factor);
    let by_other = between(other);

    (
        Some(by_factor),
        Some(by_other),
        Some((1.0 - by_factor - by_other).max(0.0)),
    )
}

/// How a row of tabs looks: the classes of the row and of a tab.
struct TabStyle {
    row: &'static str,
    tab: &'static str,
}

/// `panes`, each a label as markup and its content, as tabs: a row of the
/// labels and the content of the chosen one under it, the first to begin
/// with. The tabs are radio buttons sharing `field`, which also names their
/// ids, their panes and the class of the box scoping their rules, so the file
/// needs no script to switch them and rows nest on fields of their own.
fn tabs(field: &str, style: &TabStyle, panes: &[(String, String)]) -> String {
    let scope = format!("{field}s");
    let mut row = String::new();
    let mut content = String::new();
    let mut rules = String::new();
    for (index, (label, pane)) in panes.iter().enumerate() {
        let checked = if index == 0 { " checked" } else { "" };
        row.push_str(&format!(
            "<span><input type=\"radio\" name=\"{field}\" id=\"{field}-{index}\" class=\"peer sr-only\"{checked}>\
             <label for=\"{field}-{index}\" class=\"{}\">{label}</label></span>",
            style.tab
        ));
        content.push_str(&format!("<div class=\"{field}-pane-{index}\">{pane}</div>"));
        rules.push_str(&format!(
            ".{scope}:not(:has(#{field}-{index}:checked)) .{field}-pane-{index}{{display:none}}"
        ));
    }

    format!(
        "<div class=\"{scope}\"><style>{rules}</style><div class=\"{}\">{row}</div>{content}</div>",
        style.row
    )
}

/// The table `table` of `groups` over `columns`, one row per name, its
/// headers sorting it, arriving sorted by the score when it has one.
fn group_table(
    groups: &[Group],
    table: &str,
    first: &str,
    name: Name,
    columns: &[Column],
) -> String {
    let mut headers = vec![first];
    headers.extend(columns.iter().map(|column| column.header()));
    let score = columns
        .iter()
        .position(|column| matches!(column, Column::Score))
        .map(|column| column + MODEL_CELLS);

    let rows = groups
        .iter()
        .map(|group| {
            let mut row = vec![group_cell(group, name)];
            row.extend(columns.iter().map(|column| column.cell(&group.sum)));
            row
        })
        .collect();

    views::sorted_table(table, score, &headers, rows, Some(NO_MODELS_NOTE))
}

/// The name of `group` along `name`, as a cell: in mono, with the thinking
/// level muted after it.
fn group_cell(group: &Group, name: Name) -> String {
    let level = match group.setup.thinking.as_deref() {
        Some(thinking) if !thinking.is_empty() => format!(
            " <span class=\"{}\">{}</span>",
            views::MUTED_CLASSES,
            views::escape(thinking)
        ),
        _ => String::new(),
    };

    format!(
        "<span class=\"{} whitespace-nowrap\">{}{level}</span>",
        views::MONO_CLASSES,
        views::escape(name(&group.setup))
    )
}

/// The line of a group on a chart, without its points yet, in its colour.
fn group_series(group: &Group) -> chart::Series {
    chart::Series {
        label: group.key.clone(),
        hover: group.key.clone(),
        hue: group.hue,
        face: String::new(),
        points: Vec::new(),
    }
}

/// The share of every model's runs that had passed by every share of the
/// budget, one stepped line per model from nothing to the end of the budget.
fn pass_curve(groups: &[Group]) -> String {
    let series: Vec<chart::Series> = groups
        .iter()
        .map(|group| {
            let mut series = group_series(group);
            let mut shares = group.sum.first_pass_shares.clone();
            shares.sort_by(f64::total_cmp);
            series.points.push(chart::Point {
                x: 0.0,
                y: 0.0,
                hover: String::new(),
            });
            for (count, share) in shares.iter().enumerate() {
                let passed = count + 1;
                series.points.push(chart::Point {
                    x: share * PERCENT,
                    y: passed as f64 / group.sum.runs as f64 * PERCENT,
                    hover: format!(
                        "{}, {passed} of {} runs passed by {:.0}% of the budget",
                        series.label,
                        group.sum.runs,
                        share * PERCENT
                    ),
                });
            }
            let reached = series.points.last().map_or(0.0, |point| point.y);
            series.points.push(chart::Point {
                x: PERCENT,
                y: reached,
                hover: String::new(),
            });
            series
        })
        .collect();

    views::chart_panel(
        "passes over the budget",
        "the share of the runs that had a passing push by every share of the budget, over the finished rounds of the chosen tournaments, a curve climbing early for one that passes fast",
        &chart::lines(
            &series,
            &chart::Axis::percent().titled("share of the budget spent"),
            &chart::Axis::percent().titled("share of the runs passed"),
            chart::Shape::Stepped,
            None,
            chart::WIDE_WIDTH,
            NO_MODELS_NOTE,
        ),
    )
}

/// The score of every group against `value`, one mark per group, for the
/// groups with a score and a value, on the horizontal `axis` up to the
/// largest value.
fn scatter(
    groups: &[Group],
    about: &Scatter,
    axis: impl Fn(f64) -> chart::Axis,
    value: impl Fn(&Sum) -> Option<f64>,
    detail: impl Fn(f64) -> String,
) -> String {
    let mut top = 0.0f64;
    let series: Vec<chart::Series> = groups
        .iter()
        .filter_map(|group| {
            let score = group.sum.score()?;
            let x = value(&group.sum)?;
            top = top.max(x);
            let mut series = group_series(group);
            series.points.push(chart::Point {
                x,
                y: score * PERCENT,
                hover: format!("{}, score {score:.2}, {}", series.label, detail(x)),
            });
            Some(series)
        })
        .collect();

    views::chart_panel(
        about.title,
        about.tooltip,
        &chart::lines(
            &series,
            &axis(top).titled(about.measure),
            &chart::Axis::percent().titled("share of the rounds won"),
            chart::Shape::Scatter,
            Some(&chart::Quadrants {
                labels: about.quadrants.map(str::to_string),
            }),
            chart::NARROW_WIDTH,
            NO_MODELS_NOTE,
        ),
    )
}

/// One row per model: its score over every chosen tournament, then in each
/// of them, then what a run that passed cost it in dollars and in tokens.
fn tournaments_table(records: &[ava_wire::Tournament], played: &[Played]) -> String {
    let mut headers: Vec<String> = vec![MODEL_HEADER.to_string(), OVERALL_HEADER.to_string()];
    headers.extend(records.iter().map(|record| {
        format!(
            "#{}|the share of the rounds against other agents won in {}, half for a draw",
            record.name, record.name
        )
    }));
    headers.push(COST_PER_SUCCESS_HEADER.to_string());
    headers.push(TOKENS_PER_SUCCESS_HEADER.to_string());
    let headers: Vec<&str> = headers.iter().map(String::as_str).collect();

    let score_cell = |group: Option<&Group>| {
        group
            .and_then(|group| group.sum.score())
            .map(|score| format!("{score:.2}"))
            .unwrap_or_default()
    };
    let rows = grouped(played, model_key)
        .iter()
        .map(|model| {
            let mut row = vec![group_cell(model, model_name), score_cell(Some(model))];
            for index in 0..records.len() {
                let own = grouped(
                    played.iter().filter(|run| {
                        run.tournament == index && model_key(&run.setup) == model.key
                    }),
                    model_key,
                );
                row.push(score_cell(own.first()));
            }
            row.push(
                model
                    .sum
                    .dollars_per_pass()
                    .map(usage::money)
                    .unwrap_or_default(),
            );
            row.push(count_label(model.sum.tokens_per_pass()));
            row
        })
        .collect();

    views::sorted_table(
        TOURNAMENTS_TABLE,
        Some(OVERALL_COLUMN),
        &headers,
        rows,
        Some(NO_MODELS_NOTE),
    )
}

/// The score of every model in every chosen tournament as bars, one slot per
/// tournament with a bar per model in it.
fn tournaments_chart(
    records: &[ava_wire::Tournament],
    models: &[Group],
    played: &[Played],
) -> String {
    let series: Vec<chart::Series> = models
        .iter()
        .map(|model| {
            let mut series = group_series(model);
            for (index, record) in records.iter().enumerate() {
                let own = grouped(
                    played.iter().filter(|run| {
                        run.tournament == index && model_key(&run.setup) == model.key
                    }),
                    model_key,
                );
                if let Some(score) = own.first().and_then(|own| own.sum.score()) {
                    series.points.push(chart::Point {
                        x: index as f64,
                        y: score * PERCENT,
                        hover: format!("{} in {}, score {score:.2}", model.key, record.name),
                    });
                }
            }
            series
        })
        .collect();
    let horizontal = chart::Axis {
        min: -SLOT_MARGIN,
        max: records.len() as f64 - 1.0 + SLOT_MARGIN,
        ticks: records
            .iter()
            .enumerate()
            .map(|(index, record)| (index as f64, record.name.clone()))
            .collect(),
        title: String::new(),
        icons: records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                let (logo, kind) = views::game_logo(&record.game)?;
                Some((index as f64, data_uri(kind, &logo)))
            })
            .collect(),
    };

    views::chart_panel(
        "score by tournament",
        "the share of the rounds every model won in every chosen tournament, one bar per model in the slot of the tournament",
        &chart::lines(
            &series,
            &horizontal,
            &chart::Axis::percent().titled("share of the rounds won"),
            chart::Shape::Bars,
            None,
            chart::WIDE_WIDTH,
            NO_MODELS_NOTE,
        ),
    )
}

/// `value` of every model as one bar in a slot of its own, the smallest
/// leftmost, for the models with a value.
fn efficiency_chart(
    models: &[Group],
    title: &str,
    tooltip: &str,
    measure: &str,
    value: impl Fn(&Sum) -> Option<f64>,
    detail: impl Fn(f64) -> String,
) -> String {
    let mut ranked: Vec<(f64, &Group)> = models
        .iter()
        .filter_map(|model| Some((value(&model.sum)?, model)))
        .collect();
    ranked.sort_by(|left, right| left.0.total_cmp(&right.0));
    let top = ranked.last().map_or(0.0, |(value, _)| *value);

    let series: Vec<chart::Series> = ranked
        .iter()
        .enumerate()
        .map(|(slot, (value, model))| {
            let mut series = group_series(model);
            series.points.push(chart::Point {
                x: slot as f64,
                y: *value,
                hover: format!("{}, {}", model.key, detail(*value)),
            });
            series
        })
        .collect();
    let horizontal = chart::Axis {
        min: -SLOT_MARGIN,
        max: ranked.len() as f64 - 1.0 + SLOT_MARGIN,
        ticks: ranked
            .iter()
            .enumerate()
            .map(|(slot, (_, model))| (slot as f64, model.key.clone()))
            .collect(),
        title: String::new(),
        icons: Vec::new(),
    };

    views::chart_panel(
        title,
        tooltip,
        &chart::lines(
            &series,
            &horizontal,
            &chart::Axis::values(top).titled(measure),
            chart::Shape::Bars,
            None,
            chart::NARROW_WIDTH,
            NO_MODELS_NOTE,
        ),
    )
}

/// The whole document around `body`: the head of the layout with the styles
/// and the fonts inside it in place of their addresses, the sprite sheets and
/// the logos of `games` inside the body in place of theirs, and the script
/// sorting the tables, so nothing is fetched from the server.
fn document(body: &str, games: &[String]) -> String {
    let (head, _) = views::LAYOUT_TEMPLATE
        .split_once(HEAD_END)
        .expect("the layout has a head");
    let mut head = head.replace(views::TITLE_PLACEHOLDER, TITLE);
    for (name, bytes) in crate::serve::FONTS {
        head = head.replace(
            &format!("{FONT_ADDRESS_PREFIX}{name}"),
            &data_uri(crate::serve::FONT_CONTENT_TYPE, bytes),
        );
    }
    let head = head.replace(
        TAILWIND_TAG,
        &format!("<script>{}</script>", crate::serve::TAILWIND),
    );
    let mut body = body.to_string();
    for (name, bytes) in crate::serve::SPRITES {
        body = body.replace(
            &format!("{}{name}", crate::serve::SPRITE_PATH),
            &data_uri(crate::serve::SPRITE_CONTENT_TYPE, bytes),
        );
    }
    for game in games {
        let address = views::logo_address(game);
        if !body.contains(&address) {
            continue;
        }
        if let Some((logo, kind)) = views::game_logo(game) {
            body = body.replace(&address, &data_uri(kind, &logo));
        }
    }

    format!(
        "{head}{HEAD_END}<body class=\"{BODY_CLASSES}\"><main class=\"{MAIN_CLASSES}\">{body}</main><script>{}</script></body></html>",
        crate::serve::TABLE_SORT
    )
}

/// `bytes` of the content type `kind` as an address carrying them.
fn data_uri(kind: &str, bytes: &[u8]) -> String {
    format!("data:{kind};base64,{}", base64(bytes))
}

/// `label` with `title` behind its hover.
fn titled(label: &str, title: &str) -> String {
    format!("<span title=\"{}\">{label}</span>", views::escape(title))
}

/// `label` with the number its column sorts it by, since a label carrying a
/// unit is another number to a sort reading the text of the cell.
fn valued(value: f64, label: &str) -> String {
    format!(
        "<span {}=\"{value}\">{label}</span>",
        views::SORT_VALUE_FIELD
    )
}

/// A count of seconds as a span, nothing for none.
fn seconds_label(seconds: Option<f64>) -> String {
    seconds
        .map(|seconds| valued(seconds, &usage::span(seconds as u64)))
        .unwrap_or_default()
}

/// A count in thousands or millions, nothing for none.
fn count_label(count: Option<f64>) -> String {
    count
        .map(|count| tokens_label(count as u64))
        .unwrap_or_default()
}

/// A share as whole percent, nothing for none.
fn percent_label(share: Option<f64>) -> String {
    share
        .map(|share| format!("{:.0}%", share * PERCENT))
        .unwrap_or_default()
}

/// A count of tokens in thousands or millions, the count itself behind the hover.
fn tokens_label(count: u64) -> String {
    let compact = match count as f64 {
        tokens if tokens >= MILLION => format!("{:.1}M", tokens / MILLION),
        tokens if tokens >= THOUSAND => format!("{:.0}k", tokens / THOUSAND),
        _ => return count.to_string(),
    };

    valued(count as f64, &titled(&compact, &count.to_string()))
}

/// The mean of `values`, nothing over none.
fn mean(values: &[f64]) -> Option<f64> {
    ratio(values.iter().sum(), values.len() as f64)
}

/// `bytes` in base64, for a font inside a stylesheet.
fn base64(bytes: &[u8]) -> String {
    let mut encoded =
        String::with_capacity(bytes.len().div_ceil(BASE64_BLOCK_BYTES) * BASE64_BLOCK_CHARACTERS);
    for chunk in bytes.chunks(BASE64_BLOCK_BYTES) {
        let mut block = [0u8; BASE64_BLOCK_BYTES];
        block[..chunk.len()].copy_from_slice(chunk);
        let bits = u32::from_be_bytes([0, block[0], block[1], block[2]]);
        for index in 0..BASE64_BLOCK_CHARACTERS {
            if index <= chunk.len() {
                let shift = BASE64_BITS * (BASE64_BLOCK_CHARACTERS - 1 - index) as u32;
                encoded.push(BASE64_ALPHABET[(bits >> shift) as usize & 0x3f] as char);
            } else {
                encoded.push(BASE64_PAD);
            }
        }
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::{Played, Sum};

    fn played(passed: Option<u64>, cost: Option<f64>, won: u64, lost: u64) -> Played {
        Played {
            tournament: 0,
            setup: ava_wire::Setup {
                agent: ava_wire::Agent {
                    harness: "pi".to_string(),
                    model: "m".to_string(),
                },
                thinking: Some("high".to_string()),
                backend: None,
                name: None,
            },
            limit_seconds: 1000,
            cost,
            metrics: Some(ava_wire::Metrics {
                output_tokens: 3000,
                request_seconds: 450.0,
                ..ava_wire::Metrics::default()
            }),
            first_pass: passed,
            first_pass_tokens: passed.map(|_| 1000),
            high_score: passed,
            high_score_tokens: passed.map(|_| 1000),
            rounds: ava_wire::Tally {
                won,
                drawn: 0,
                lost,
            },
        }
    }

    #[test]
    fn a_sum_derives_its_measures_from_the_runs() {
        let mut sum = Sum::default();
        sum.add(&played(Some(250), Some(2.0), 3, 1));
        sum.add(&played(None, Some(4.0), 0, 4));
        sum.add(&played(Some(750), None, 1, 3));

        assert_eq!(sum.runs, 3);
        assert_eq!(sum.passed, 2);
        assert_eq!(sum.dollars_per_pass(), Some(3.0));
        assert_eq!(sum.tokens_per_pass(), Some(4500.0));
        assert_eq!(sum.score(), Some(4.0 / 12.0));
        assert_eq!(sum.dollars_per_run(), Some(3.0));
        assert_eq!(sum.dollars_per_round_won(), Some(1.5));
        assert_eq!(super::mean(&sum.first_pass_shares), Some(0.5));
        assert_eq!(super::mean(&sum.first_pass_seconds), Some(500.0));
        assert_eq!(super::mean(&sum.high_score_tokens), Some(1000.0));
        assert_eq!(sum.tokens_per_round_won(), Some(2250.0));
        assert_eq!(sum.thousand_output_per_run(), Some(3.0));
    }

    #[test]
    fn groups_sort_by_score_then_by_price() {
        let runs = vec![
            played(Some(1), Some(5.0), 1, 1),
            played(Some(1), Some(1.0), 2, 0),
        ];
        let mut cheaper = runs;
        cheaper[1].setup.agent.harness = "codex".to_string();
        let groups = super::grouped(&cheaper, super::agent_key);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].setup.agent.harness, "codex");
    }

    #[test]
    fn the_labels_read_as_the_columns_say() {
        assert_eq!(super::tokens_label(999), "999");
        assert!(super::tokens_label(262_798).contains(">263k<"));
        assert!(super::tokens_label(27_449_604).contains(">27.4M<"));
        assert_eq!(super::percent_label(Some(0.4251)), "43%");
        // A label carrying a unit sorts by the number behind it, not by its text.
        assert!(super::tokens_label(262_798).contains("data-value=\"262798\""));
        assert!(super::seconds_label(Some(4080.0)).contains(">1h 8m<"));
        assert!(super::seconds_label(Some(4080.0)).contains("data-value=\"4080\""));
        assert_eq!(super::seconds_label(None), "");
        assert_eq!(super::percent_label(None), "");
        assert_eq!(super::mean(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(super::mean(&[]), None);
        let piece = |label: &str, names: &[&str]| super::Part {
            label: label.to_string(),
            names: names.iter().map(|name| name.to_string()).collect(),
        };
        assert_eq!(
            super::file_name(&[piece("", &["a", "b"])]),
            "report-a-b.html"
        );
        assert_eq!(
            super::file_name(&[piece("max", &["a"]), piece("high", &["b"])]),
            "report-max-high.html"
        );
        assert_eq!(super::file_name(&[]), "report.html");
    }

    #[test]
    fn base64_pads_the_last_block() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foo"), "Zm9v");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn links_point_inside_the_file() {
        let body = "<a href=\"/run/x\">x</a><a href=\"/run/x/run.json\">run.json</a><a href=\"/run/xy\">xy</a>";

        assert_eq!(
            super::pointed_inside(body, "/run/x", "run-x"),
            "<a href=\"#run-x\">x</a><a href=\"/run/x/run.json\">run.json</a><a href=\"/run/xy\">xy</a>"
        );
    }

    #[test]
    fn the_document_fetches_nothing_from_the_server() {
        let (sheet, _) = crate::serve::SPRITES[0];
        let body = format!(
            "<p>body</p><span style=\"background-image:url('{}{sheet}')\"></span>",
            crate::serve::SPRITE_PATH
        );
        let document = super::document(&body, &[]);

        assert!(document.starts_with("<!doctype html>"));
        assert!(document.contains("data:font/woff2;base64,"));
        assert!(document.contains("url('data:image/png;base64,"));
        assert!(!document.contains("/assets/"));
        assert!(document.contains("<title>Agent vs Agent"));
        assert!(document.contains("table[data-sortable]"));
        assert!(document.ends_with("</html>"));
    }
}
