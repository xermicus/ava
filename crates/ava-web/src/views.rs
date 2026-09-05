//! The pages of the web interface, rendered from the run and tournament
//! records on disk, the registry and the games folder.

use ava_game::scoring::Scoring;
use ava_run::{docker, process, registry, runs, tournament, usage};

const GAMES_DIRECTORY: &str = "games";
const TASK_FILE: &str = "task.md";
const INSTRUCTIONS_FILE: &str = "README.md";

/// What the start panel offers preselected on a fresh page.
const DEFAULT_GAME: &str = "sanity-check";
const DEFAULT_THINKING: &str = "medium";

/// What the analysis settings offer preselected.
const DEFAULT_ANALYST: &str = "claude";
const DEFAULT_ANALYST_MODEL: &str = "claude-sonnet-5";
const DEFAULT_ANALYST_THINKING: &str = "medium";

/// The files of a run the raw file routes hand out, and nothing else.
const RUN_FILES: [&str; 11] = [
    docker::MONITOR_FILE,
    docker::AGENT_LOG,
    docker::ANALYSIS_FILE,
    docker::ANALYSIS_LOG,
    docker::ANALYSIS_ACCESS_LOG,
    docker::ANALYSIS_ERROR_LOG,
    docker::SCORE_LOG,
    docker::RUN_FILE,
    docker::ACCESS_LOG,
    docker::ERROR_LOG,
    docker::SCORE_ERROR_LOG,
];

/// The fields of the run record that are shown on their own rather than among
/// the parameters.
const RUN_RECORD_SECTIONS: [&str; 2] = ["attempts", "metrics"];

/// The console of the fights of one round, handed out by the tournament file route.
const ROUND_LOG_PREFIX: &str = "round-";
const ROUND_LOG_SUFFIX: &str = ".log";

/// How much of the agent console the run page shows inline.
const CONSOLE_TAIL_BYTES: usize = 16 * 1024;

const LAYOUT_TEMPLATE: &str = include_str!("../assets/web-layout.html");
const HEADING_PLACEHOLDER: &str = "__AVA_HEADING__";
const BODY_PLACEHOLDER: &str = "__AVA_BODY__";

/// A `#` prefix on a header right-aligns that column for numbers.
const NUMERIC_MARKER: char = '#';

/// A `*` prefix on a header marks the column taking the slack of the row.
const SLACK_MARKER: char = '*';

/// Whatever follows a `|` in a header is the tooltip explaining that column
/// rather than part of its title.
const TOOLTIP_SEPARATOR: char = '|';

/// The unified runs table, holding pending, live and finished runs alike.
const RUN_HEADERS: [&str; 11] = [
    "RUN|the run directory under runs/, how long ago it started, and the tournament seat it plays",
    "STATE|live or the last call while the run goes, whether a push passed the verifier once it is over",
    "ANALYSIS|whether an analyst was run over the finished run: analyzing, analyzed or failed",
    "GAME|the game that was played",
    "MODEL|the model under test",
    "HARNESS|the harness driving the model, with the thinking level it was asked for",
    "*TIME|seconds spent of the time budget, red once the whole budget is gone",
    "#PUSHES|the pushes to the task branch the verifier graded",
    "#CUT|requests a model answered without ever reporting usage, so the stream was cut short \
     upstream",
    "*POINTS|the entry of record ranked on the 0 to 10000 scale every game ranks in, once the run \
     is over",
    "",
];
const NO_RUNS_NOTE: &str = "no runs yet, start one above";
const NO_LIMITS_NOTE: &str = "no backend reported its limits";
const NO_TOURNAMENTS_NOTE: &str = "no tournaments yet, open one above";
const NO_SEATS_NOTE: &str = "no seats yet, seat an agent below";
const NO_REPORT_NOTE: &str = "the record holds neither a report nor a reason";
const IMAGE_FORMAT: &str = "{{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}";
const IMAGE_PREFIX: &str = "ava/";

/// A card holds one table or one form, so every block on a page shares the
/// same edges and corners.
const CARD_CLASSES: &str = "rounded-lg border border-neutral-800 bg-neutral-900";

/// Every table spans its card. The columns pack on one gutter, shrunk to
/// their content, and one of them takes the slack, so the columns before it
/// start on the left edge, the ones after it end on the right edge and every
/// gap stays the same width. Without a marked column the last one takes the
/// slack.
const TABLE_CLASSES: &str = "w-full border-collapse";
const PACKED_COLUMN_CLASSES: &str = "w-px whitespace-nowrap px-2 first:pl-4 last:pr-4";
const SLACK_COLUMN_CLASSES: &str = "px-2 first:pl-4 last:pr-4";
const HEADER_CLASSES: &str = "text-xs font-medium uppercase tracking-wider text-neutral-500 py-2.5";

/// A title with a tooltip behind it.
const TOOLTIP_CLASSES: &str = "cursor-help underline decoration-dotted decoration-neutral-700 \
     underline-offset-4 hover:text-neutral-300 hover:decoration-neutral-500 transition-colors";
const CELL_CLASSES: &str = "py-2.5 border-t border-neutral-800 align-middle";
const ROW_CLASSES: &str = "hover:bg-neutral-800/40 transition-colors";
const NUMERIC_CLASSES: &str = "text-right font-mono tabular-nums";
const EMPTY_ROW_CLASSES: &str =
    "px-4 py-8 text-center text-neutral-500 border-t border-neutral-800";

const TITLE_CLASSES: &str = "text-sm font-semibold text-neutral-100 mt-8 mb-3";

/// The first title of a page rests on the padding of the layout.
const FIRST_TITLE_CLASSES: &str = "text-sm font-semibold text-neutral-100 mb-3";
const NOTE_CLASSES: &str = "text-neutral-400";
const MUTED_CLASSES: &str = "text-neutral-500";
const MONO_CLASSES: &str = "font-mono";
const LINK_CLASSES: &str = "font-mono text-indigo-300 hover:text-indigo-200 transition-colors";
const CONSOLE_CLASSES: &str = "rounded-lg border border-neutral-800 bg-neutral-950 p-4 text-xs \
     font-mono text-neutral-300 whitespace-pre overflow-x-auto";
/// A section title that folds its section.
const COLLAPSIBLE_TITLE_CLASSES: &str = "cursor-pointer list-none [&::-webkit-details-marker]:hidden \
     text-sm font-semibold text-neutral-100 mt-8 mb-3";

/// The renderer caps its prose at a reading width. Inside a box that is the
/// width, the prose fills the box and wraps at its edge.
const FULL_WIDTH_PROSE: &str = "[&_*]:max-w-none";
const SUMMARY_CLASSES: &str = "cursor-pointer list-none [&::-webkit-details-marker]:hidden \
     text-neutral-400 hover:text-neutral-200 transition-colors";

/// One height for every form control, so a row of them shares a baseline.
const CONTROL_HEIGHT: &str = "h-9";
const BUTTON_CLASSES: &str =
    "rounded-md bg-indigo-500 hover:bg-indigo-400 px-4 font-medium text-white transition-colors";
const STOP_CLASSES: &str = "rounded-md border border-red-500/40 text-red-400 hover:bg-red-500/10 \
     px-2.5 py-1 text-xs font-medium transition-colors";
const FIELD_CLASSES: &str = "w-full rounded-md border border-neutral-700 bg-neutral-950 px-3 \
     text-neutral-100 focus:outline-none focus:border-indigo-500 focus:ring-2 \
     focus:ring-indigo-500/30 transition";
const LABEL_CLASSES: &str = "block text-xs font-medium text-neutral-400 mb-1.5";

/// The pills marking a state, one tint per outcome.
const PILL_CLASSES: &str =
    "inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-xs font-medium";
const LIVE_PILL: &str = "bg-emerald-500/10 text-emerald-400";
const PASSED_PILL: &str = "bg-emerald-500/10 text-emerald-400";
const FAILED_PILL: &str = "bg-orange-500/10 text-orange-400";
const BROKEN_PILL: &str = "bg-red-500/10 text-red-400";
const STARTING_PILL: &str = "bg-amber-500/10 text-amber-400";
const ANALYZED_PILL: &str = "bg-indigo-500/10 text-indigo-300";
const NEUTRAL_PILL: &str = "bg-neutral-800 text-neutral-400";

/// The tints of a tally, by who came out ahead.
const AHEAD_CLASSES: &str = "text-emerald-400";
const BEHIND_CLASSES: &str = "text-red-400";
const LEVEL_CLASSES: &str = "text-neutral-300";

/// The meters: a track, a fill and a mono label.
const METER_TRACK_CLASSES: &str =
    "h-1.5 flex-1 min-w-12 rounded-full bg-neutral-800 overflow-hidden";

/// The labels beside the meters have one width per kind, so the tracks of
/// one column start and end on the same lines.
const POINTS_LABEL_WIDTH: &str = "w-12";
const ELAPSED_LABEL_WIDTH: &str = "w-24";
const USAGE_LABEL_WIDTH: &str = "w-16";
const USAGE_FILL: &str = "bg-amber-500";
const WAIT_FILL: &str = "bg-sky-500";
const RESET_LABEL_WIDTH: &str = "w-44";
const POINTS_FILL: &str = "bg-amber-500";

/// The values of a key value table: code and numbers in mono, sentences in
/// the text face.
const MONO_VALUE_CLASSES: &str = "font-mono text-neutral-200";
const TEXT_VALUE_CLASSES: &str = "text-neutral-200";

/// The time meter, tinted by whether the budget held.
const TIME_SPENT_FILL: &str = "bg-red-500";
const TIME_LEFT_FILL: &str = "bg-emerald-500";

/// The tiles summarizing a run, one figure each.
const TILE_CLASSES: &str = "rounded-lg border border-neutral-800 bg-neutral-900 px-4 py-3";
const TILE_LABEL_CLASSES: &str = "text-xs font-medium uppercase tracking-wider text-neutral-500";
const TILE_VALUE_CLASSES: &str =
    "mt-1 text-lg font-semibold text-neutral-100 font-mono tabular-nums";
const TILE_TEXT_CLASSES: &str = "mt-1 text-sm font-semibold text-neutral-100 font-mono break-all";
const TILE_DETAIL_CLASSES: &str = "mt-1 text-xs text-neutral-500 break-all";
/// A tile listing names and links, one per line, none broken mid-word.
const TILE_LIST_CLASSES: &str = "mt-1 text-sm text-neutral-100 font-mono";
const TILE_GRID_CLASSES: &str = "mt-3 grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-3";

/// The games page: a grid of cards, one per game, each folding out to the
/// full width of the grid.
const STOREFRONT_CLASSES: &str =
    "grid grid-cols-1 lg:grid-cols-2 2xl:grid-cols-3 gap-4 items-start";
const GAME_CARD_CLASSES: &str = "group rounded-lg border border-neutral-800 bg-neutral-900 \
     overflow-hidden open:col-span-full";
const GAME_FACE_CLASSES: &str = "block cursor-pointer list-none [&::-webkit-details-marker]:hidden \
     p-4 hover:bg-neutral-800/40 transition-colors focus-visible:outline-none \
     focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-indigo-500/50";
const GAME_NAME_CLASSES: &str = "font-mono text-sm font-semibold text-neutral-100";
const GAME_TAGLINE_CLASSES: &str = "flex items-baseline justify-between gap-3 mt-1";
const FACT_VALUE_CLASSES: &str = "font-mono tabular-nums text-neutral-100";
const CHEVRON_CLASSES: &str = "h-4 w-4 shrink-0 text-neutral-500 transition-transform \
     motion-reduce:transition-none group-open:rotate-180";
const GAME_BODY_CLASSES: &str = "border-t border-neutral-800 px-4 pb-4";
/// Between the tasks of the turns of a game with several.
const TASK_SEPARATOR: &str = "<hr class=\"my-6 border-neutral-800\">";

/// The pill naming one turn of a game with several.
const TURN_PILL: &str = "bg-neutral-800 text-neutral-400";

/// The graph of a round: nodes of one size on a grid of turns and seats,
/// edges bending between the columns, text set in the font size of the page.
const GRAPH_NODE_WIDTH: f64 = 260.0;
const GRAPH_NODE_HEIGHT: f64 = 54.0;
const GRAPH_COLUMN_GAP: f64 = 88.0;
const GRAPH_ROW_GAP: f64 = 12.0;
const GRAPH_HEADER_HEIGHT: f64 = 28.0;
const GRAPH_PADDING: f64 = 2.0;
const GRAPH_TEXT_INSET: f64 = 12.0;
const GRAPH_LINE_ONE: f64 = 21.0;
const GRAPH_LINE_TWO: f64 = 41.0;
const GRAPH_DOT_LIFT: f64 = 4.0;
const GRAPH_STATE_WIDTH: f64 = 62.0;
const GRAPH_FONT_SIZE: u32 = 12;
const GRAPH_LABEL_CHARS: usize = 34;
const GRAPH_CLASSES: &str = "font-sans";
const GRAPH_HEADER_TEXT_CLASSES: &str = "fill-neutral-500 font-mono";
const GRAPH_NODE_CLASSES: &str =
    "fill-neutral-950 stroke-neutral-800 hover:stroke-neutral-600 transition-colors";
const GRAPH_LABEL_CLASSES: &str = "fill-neutral-300";
const GRAPH_POINTS_CLASSES: &str = "fill-amber-400 font-mono";
const GRAPH_RUN_CLASSES: &str = "fill-indigo-300 font-mono";
const GRAPH_STATE_CLASSES: &str = "font-medium";
const GRAPH_EDGE_CLASSES: &str = "stroke-neutral-700";
/// The turn the attacks of a record from before the turns count as.
const LEGACY_ATTACK_TURN: usize = 1;

/// The cover of a card shows the entry of record as it is: the first bytes of
/// a binary as a grid of cells shaded by their value, so the size and the
/// shape of the file are the picture, and a text file as its text.
const COVER_CLASSES: &str = "flex h-20 w-20 shrink-0 rounded-md border border-neutral-800 \
     bg-neutral-950 overflow-hidden";
const COVER_EMPTY_CLASSES: &str =
    "h-20 w-20 shrink-0 rounded-md border border-dashed border-neutral-800";
const COVER_ART_CLASSES: &str = "block h-full w-full text-neutral-200";
const COVER_IMAGE_CLASSES: &str = "block h-full w-full object-cover";
/// The image a game folder may hold for its cover, the first one found.
const COVER_FILES: [(&str, &str); 4] = [
    ("cover.png", "image/png"),
    ("cover.svg", "image/svg+xml"),
    ("cover.webp", "image/webp"),
    ("cover.jpg", "image/jpeg"),
];
const COVER_TEXT_CLASSES: &str = "block w-full p-1.5 font-mono text-[10px] leading-tight \
     text-neutral-300 whitespace-pre overflow-hidden";
const COVER_SIDE: usize = 16;
const COVER_BYTES: usize = COVER_SIDE * COVER_SIDE;
/// A text entry no longer than this reads as text on the cover.
const TEXT_ENTRY_LIMIT: u64 = 4096;
/// The facts keep a reading width when the card spans the grid.
const FACTS_CLASSES: &str = "flex-1 min-w-0 max-w-xl flex flex-col gap-1.5";
const FACT_ROW_CLASSES: &str = "flex items-center gap-3 h-5";
/// Who holds the record, under its row, kept as a line even when empty so
/// every face stands the same height.
const HOLDER_CLASSES: &str = "block h-4 mt-1 pl-[4.25rem] text-xs text-neutral-500 truncate";
const FACT_LABEL_CLASSES: &str = "w-14 shrink-0 text-xs text-neutral-500";

/// A tile with nothing to show says why, quietly.
const PLACEHOLDER_CLASSES: &str = "text-sm font-normal text-neutral-500";
const AFTER_THE_RUN: &str = "after the run";
const NOT_RECORDED: &str = "not recorded";
const NO_ENTRY: &str = "no entry";

/// A passing run from before entries were kept left no entry file.
const NOT_KEPT: &str = "not kept";
const UNRANKED: &str = "unranked";
const UNFINISHED: &str = "unfinished";
const NOT_ANALYZED: &str = "none";

/// The figures of a live run count what its proxy logged up to the last look.
const SO_FAR: &str = "so far";

/// What the last action left for the page to show.
pub(crate) struct Notice {
    /// The action went ahead.
    pub started: Option<String>,
    /// The action was refused, and why.
    pub refused: Option<String>,
}

impl Notice {
    /// The notice as a banner, tinted by outcome.
    fn render(&self) -> String {
        if let Some(refused) = &self.refused {
            return format!(
                "<p class=\"mt-4 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-red-300\">{}</p>",
                escape(refused)
            );
        }

        match &self.started {
            Some(action) => format!(
                "<p class=\"mt-4 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-emerald-300\">{}</p>",
                escape(action)
            ),
            None => String::new(),
        }
    }
}

/// What a form shows selected, carried through the action redirect so a
/// submission does not reset it.
#[derive(Default)]
pub(crate) struct Selection {
    pub fields: Vec<(String, String)>,
}

impl Selection {
    /// The carried value of `field`, or `default` without one.
    fn get<'a>(&'a self, field: &str, default: &'a str) -> &'a str {
        self.fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value.as_str())
            .unwrap_or(default)
    }

    /// The carried agent under `prefix`, or the `defaults`.
    fn agent<'a>(&'a self, prefix: &str, defaults: [&'a str; 3]) -> [&'a str; 3] {
        let mut chosen = defaults;
        for (field, chosen) in crate::serve::AGENT_FIELDS.iter().zip(chosen.iter_mut()) {
            *chosen = self.get(&format!("{prefix}{field}"), chosen);
        }
        chosen
    }
}

/// A run the server was asked to start whose containers are not up yet.
#[derive(Clone)]
pub(crate) struct Pending {
    pub agent: String,
    pub model: String,
    pub game: String,
    pub thinking: String,
    pub parallel: u64,
    pub started: u64,
}

/// One run directory with everything the views read out of it.
struct RunEntry {
    name: String,
    run: ava_wire::Run,
    live: bool,
    /// What became of the analysis of the run.
    analysis: Analysis,
    /// The newest heartbeat of the run loop, for a live run.
    monitor: Option<serde_json::Value>,
    /// The pushes graded so far: the record once the run is over, the output
    /// of the scoring container while it is live.
    attempts: Vec<ava_wire::Attempt>,
    /// The entry of record, once the run is over and kept one.
    record: Option<runs::Entry>,
    /// The metrics of the run: the record once it is over, the aggregate of
    /// what its proxy logged so far while it is live.
    metrics: Option<ava_wire::Metrics>,
    /// The tournament seat the run plays, if any.
    placement: Option<tournament::Placement>,
}

impl RunEntry {
    fn new(
        directory: &std::path::Path,
        run: ava_wire::Run,
        running: &[String],
        placements: &std::collections::HashMap<String, tournament::Placement>,
    ) -> Self {
        let name = directory
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| run.run.clone());
        let live = running.contains(&docker::scorer_container(&name));
        let record = if live {
            None
        } else {
            ava_game::find(&run.game).and_then(|game| {
                runs::entry_of_record(game, directory, runs::kept_file(game, &run))
                    .ok()
                    .flatten()
            })
        };

        let (attempts, metrics) = if live {
            live_run(&name)
                .map(|seen| (seen.attempts, seen.metrics))
                .unwrap_or_default()
        } else {
            (attempts_of(directory, &run), run.metrics.clone())
        };

        Self {
            live,
            metrics,
            analysis: analysis_of(
                directory,
                running.contains(&docker::analyst_container(&name)),
            ),
            monitor: read_json(&directory.join(docker::MONITOR_FILE)),
            attempts,
            record,
            placement: placements.get(&name).cloned(),
            name,
            run,
        }
    }

    /// The harness with its thinking level, the way an agent is referred to.
    fn agent(&self) -> String {
        agent_label(
            &self.run.harness,
            self.run.thinking.as_deref().unwrap_or(""),
        )
    }

    /// Whether any push passed the verifier.
    fn passed(&self) -> bool {
        self.attempts.iter().any(|attempt| attempt.verdict.passed)
    }

    /// The points of the entry of record, nothing for a game ranking nothing.
    fn points(&self) -> Option<u64> {
        self.record.as_ref().and_then(|entry| entry.points)
    }

    /// The run name as a link into its page.
    fn link(&self) -> String {
        run_link(&self.name)
    }

    /// Whether an analyst is up for the run.
    fn analyzing(&self) -> bool {
        matches!(self.analysis, Analysis::Analyzing)
    }

    /// The state of the analysis as a pill, or nothing when none was started.
    fn analysis_pill(&self) -> String {
        match self.analysis {
            Analysis::Analyzing => pill(STARTING_PILL, true, "analyzing"),
            Analysis::Done => pill(ANALYZED_PILL, false, "analyzed"),
            Analysis::Failed => pill(BROKEN_PILL, false, "failed"),
            Analysis::None => String::new(),
        }
    }

    /// The analysis cell of the runs table: the pill, or that there is none
    /// once the run is over and could have been analyzed.
    fn analysis_cell(&self) -> String {
        match self.analysis {
            Analysis::None if !self.live => placeholder(NOT_ANALYZED),
            _ => self.analysis_pill(),
        }
    }

    /// The points tile of the run page: the entry of record ranked, or why
    /// there is nothing to rank yet or at all.
    fn points_tile(&self) -> String {
        match self.points() {
            Some(points) => points_meter(points),
            None if self.live => placeholder(AFTER_THE_RUN),
            None if self.record.is_some() => placeholder(UNRANKED),
            None if self.passed() => placeholder(NOT_KEPT),
            None => placeholder(NO_ENTRY),
        }
    }

    /// The run cell of the runs table: the link with the age beneath it, and
    /// the tournament seat it plays.
    fn run_cell(&self) -> String {
        let mut cell = format!(
            "{}<div class=\"text-xs {MUTED_CLASSES} mt-0.5\">{} ago</div>",
            self.link(),
            usage::age(self.run.started_seconds)
        );
        if let Some(placement) = &self.placement {
            cell.push_str(&format!(
                "<div class=\"text-xs {MUTED_CLASSES} mt-0.5\">{}</div>",
                placement_label(
                    placement,
                    ava_game::find(&self.run.game).map_or(1, |game| game.turns().len())
                )
            ));
        }
        cell
    }

    /// The state of the run as a pill.
    fn state(&self) -> String {
        if self.live {
            let live = pill(
                LIVE_PILL,
                true,
                if self.last_call() {
                    "last call"
                } else {
                    "live"
                },
            );
            return if self.passed() {
                format!("{live} {}", pill(PASSED_PILL, false, "passed"))
            } else {
                live
            };
        }

        if self.passed() {
            pill(PASSED_PILL, false, "passed")
        } else if self.run.finished_seconds.is_some() {
            pill(FAILED_PILL, false, "failed")
        } else {
            pill(BROKEN_PILL, false, "unfinished")
        }
    }

    /// Whether the run is past its limit and answering its last call, which is
    /// why its elapsed meter sits full while it is still live.
    fn last_call(&self) -> bool {
        self.monitor
            .as_ref()
            .and_then(|heartbeat| heartbeat.get("last_call"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }

    /// The seconds a live run has used of its budget.
    ///
    /// The heartbeat of the run loop is the elapsed time of record: the loop
    /// clock pauses with a sleeping host, which wall clock arithmetic misses.
    /// It counts from the start of the run, so the last call restart does not
    /// send it back to zero.
    fn elapsed(&self) -> u64 {
        match &self.monitor {
            Some(heartbeat) => number(heartbeat, "elapsed_seconds"),
            None => usage::epoch_now().saturating_sub(self.run.started_seconds),
        }
        .min(self.run.limit_seconds)
    }

    /// The time cell of the runs table: the same meter against the budget for
    /// every run, spent while live, taken once over, nothing for a run that broke.
    fn time_cell(&self) -> String {
        let spent = if self.live {
            self.elapsed()
        } else if let Some(wall) = self.run.wall_seconds() {
            wall
        } else {
            return String::new();
        };
        let limit = self.run.limit_seconds;

        meter(
            spent,
            limit,
            time_fill(spent, limit),
            &format!("{spent}/{limit}s"),
            ELAPSED_LABEL_WIDTH,
        )
    }

    /// The requests a model answered without ever reporting their usage.
    ///
    /// Blank when there were none. It is what tells a run that spent its
    /// budget on streams which never finished apart from one where the model
    /// simply did not pass the task, since both rank nowhere.
    fn truncated_cell(&self) -> String {
        match self
            .metrics
            .as_ref()
            .map(|metrics| metrics.truncated_requests)
        {
            Some(0) | None => String::new(),
            Some(cut) => cut.to_string(),
        }
    }

    /// The points cell of the runs table: the entry of record ranked, once
    /// the run is over. The entries stay in the scoring container while it plays.
    fn points_cell(&self) -> String {
        match self.points() {
            Some(points) => points_meter(points),
            None => String::new(),
        }
    }

    /// The button ending this run early, for a live run.
    fn stop_form(&self) -> String {
        if !self.live {
            return String::new();
        }

        format!(
            "<form method=\"post\" action=\"/run/{}/stop\"><button class=\"{STOP_CLASSES}\">stop</button></form>",
            escape(&self.name)
        )
    }

    /// The row of this run in the runs table.
    fn row(&self) -> Vec<String> {
        vec![
            self.run_cell(),
            self.state(),
            self.analysis_cell(),
            game_label(&self.run),
            escape(&self.run.model),
            self.agent(),
            self.time_cell(),
            self.attempts.len().to_string(),
            self.truncated_cell(),
            self.points_cell(),
            self.stop_form(),
        ]
    }
}

/// What became of the analysis of a run.
#[derive(Clone, Copy)]
enum Analysis {
    /// No analyst was started on the run.
    None,
    /// An analyst is up on the run.
    Analyzing,
    /// The record holds the report.
    Done,
    /// The record holds the reason the analysis failed.
    Failed,
}

/// What became of the analysis of the run in `directory`, given whether an
/// analyst is up on it.
fn analysis_of(directory: &std::path::Path, analyzing: bool) -> Analysis {
    if analyzing {
        return Analysis::Analyzing;
    }

    match runs::analysis(directory) {
        Ok(None) => Analysis::None,
        Ok(Some(record)) if record.error.is_none() && record.report().is_some() => Analysis::Done,
        _ => Analysis::Failed,
    }
}

/// The analyst behind a record: who it was, its version, its turns, the
/// seconds it took, the tokens it wrote and the cost the gateway reported.
fn analyst_rows(record: &ava_wire::Analysis) -> Vec<(String, String)> {
    let Some(analyst) = &record.analyst else {
        return Vec::new();
    };

    let mut rows = vec![
        ("analyst".to_string(), analyst.label()),
        (
            "harness version".to_string(),
            record.harness_version.clone(),
        ),
        ("turns".to_string(), record.turns.to_string()),
        (
            "seconds".to_string(),
            format!(
                "{} of {}",
                record
                    .finished_seconds
                    .saturating_sub(record.started_seconds),
                record.limit_seconds
            ),
        ),
    ];
    if let Some(metrics) = &record.metrics {
        rows.push((
            "output tokens".to_string(),
            metrics.output_tokens.to_string(),
        ));
        if metrics.gateway_cost > 0.0 {
            rows.push(("cost".to_string(), usage::money(metrics.gateway_cost)));
        }
    }
    rows
}

/// The fields of a report as label and text, the closed vocabularies spelled
/// out, the fields the analyst left empty dropped.
fn report_rows(report: &ava_wire::Report) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    if let Some(meaning) = ava_wire::meaning(&ava_wire::ATTRIBUTIONS, &report.attribution) {
        rows.push(("outcome decided by".to_string(), meaning.to_string()));
    }
    if report.failure_mode == ava_wire::OTHER_FAILURE_MODE {
        rows.push(("failure mode".to_string(), report.other_failure.clone()));
    } else if let Some(meaning) = ava_wire::meaning(&ava_wire::FAILURE_MODES, &report.failure_mode)
    {
        rows.push(("failure mode".to_string(), meaning.to_string()));
    }

    let sentences = [
        ("strategy", &report.strategy),
        ("went well", &report.went_well),
        ("agent mistakes", &report.agent_mistakes),
        ("environment issues", &report.environment_issues),
        ("decisive", &report.decisive),
        ("verification", &report.verification),
        ("pacing", &report.pacing),
        ("counterfactual", &report.counterfactual),
    ];
    rows.extend(
        sentences
            .into_iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(label, value)| (label.to_string(), value.clone())),
    );
    rows
}

/// A report on the run page: the summary, then folded behind it the fields,
/// the analysis and the analyst.
fn report_card(record: &ava_wire::Analysis, report: &ava_wire::Report) -> String {
    let block = |html: String| {
        if html.is_empty() {
            String::new()
        } else {
            format!("<div class=\"mt-3\">{html}</div>")
        }
    };
    let rows = |rows: Vec<(String, String)>| {
        if rows.is_empty() {
            String::new()
        } else {
            pairs_table(&rows, TEXT_VALUE_CLASSES)
        }
    };
    let analysis = if report.analysis.is_empty() {
        String::new()
    } else {
        ava_markdown::render(&report.analysis)
    };

    format!(
        "<div class=\"{CARD_CLASSES} {FULL_WIDTH_PROSE} px-4 py-3 mb-4\">{}\
         <details class=\"mt-3\" data-fold=\"full-analysis\"><summary class=\"{SUMMARY_CLASSES}\">the full analysis</summary>{}{}{}</details></div>",
        ava_markdown::render(summary_body(&report.summary)),
        block(rows(report_rows(report))),
        block(analysis),
        block(rows(analyst_rows(record)))
    )
}

/// The summary without the heading an analyst puts over it, since the card
/// showing it is the heading.
fn summary_body(summary: &str) -> &str {
    let trimmed = summary.trim_start();
    if !trimmed.starts_with('#') {
        return trimmed;
    }

    match trimmed.split_once('\n') {
        Some((_, rest)) if !rest.trim().is_empty() => rest,
        _ => trimmed,
    }
}

/// The landing page: the start panel and every run, newest first.
pub(crate) fn runs_page(
    notice: &Notice,
    selection: &Selection,
    pending: &[Pending],
) -> std::io::Result<String> {
    let runs = collect_runs()?;

    let mut rows: Vec<Vec<String>> = Vec::new();

    // A start shows up the moment it was asked for, and stays a starting row
    // until the runs it spawns appear on disk.
    for start in pending {
        let appeared = runs
            .iter()
            .filter(|run| {
                run.run.harness == start.agent
                    && run.run.model == start.model
                    && run.run.game == start.game
                    && run.run.started_seconds + 1 >= start.started
            })
            .count() as u64;

        for _ in appeared..start.parallel {
            rows.push(vec![
                format!(
                    "<span class=\"{MUTED_CLASSES}\">pending</span><div class=\"text-xs {MUTED_CLASSES} mt-0.5\">asked {} ago</div>",
                    usage::age(start.started)
                ),
                pill(STARTING_PILL, true, "starting"),
                String::new(),
                escape(&start.game),
                escape(&start.model),
                agent_label(&start.agent, &start.thinking),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ]);
        }
    }

    rows.extend(runs.iter().map(RunEntry::row));

    let live = runs.iter().filter(|run| run.live).count();
    let mut body = start_panel(selection)?;
    body.push_str("<div data-refresh=\"runs\">");
    body.push_str(&notice.render());
    body.push_str(&format!(
        "<p class=\"{TITLE_CLASSES}\">runs <span class=\"{NOTE_CLASSES} font-normal\">{} on disk, {live} live</span></p>{}",
        runs.len(),
        table(&RUN_HEADERS, rows, Some(NO_RUNS_NOTE))
    ));
    body.push_str("</div>");

    Ok(page("runs", &body))
}

/// The panel starting a run, offering what the registry and the games folder
/// hold, with the carried `selection` or the defaults preselected.
fn start_panel(selection: &Selection) -> std::io::Result<String> {
    let registry = registry::load()?;
    let games = games()?;
    let games = games.iter().map(String::as_str).collect::<Vec<_>>();

    let limit = selection
        .get("limit", "")
        .parse::<u64>()
        .unwrap_or(docker::Agent::DEFAULT_LIMIT_SECONDS);
    let parallel = selection
        .get("parallel", "")
        .parse::<u64>()
        .unwrap_or(docker::Agent::DEFAULT_PARALLEL_RUNS);
    let last_call = docker::LAST_CALL_SECONDS;
    let seconds_label = explained(
        "seconds",
        &format!("the whole budget, the {last_call} second last call included"),
    );

    Ok(format!(
        "<p class=\"{FIRST_TITLE_CLASSES}\">new run</p>\
         <form method=\"post\" action=\"/start\" class=\"{CARD_CLASSES} p-4 flex flex-wrap items-end gap-4\">\
         {}{}\
         <label class=\"w-24\"><span class=\"{LABEL_CLASSES}\">{seconds_label}</span>\
         <input class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" type=\"number\" name=\"limit\" value=\"{limit}\" min=\"{last_call}\"></label>\
         <label class=\"w-20\"><span class=\"{LABEL_CLASSES}\">parallel</span>\
         <input class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" type=\"number\" name=\"parallel\" value=\"{parallel}\" min=\"1\"></label>\
         <label class=\"{CONTROL_HEIGHT} flex items-center gap-2\">\
         <input type=\"checkbox\" name=\"force\" class=\"h-4 w-4 rounded accent-indigo-500\"{force}>\
         <span class=\"{NOTE_CLASSES}\">rebuild images</span></label>\
         <button class=\"{BUTTON_CLASSES} {CONTROL_HEIGHT}\">start</button>\
         <div class=\"w-full flex flex-wrap items-end gap-4\">\
         <input type=\"checkbox\" id=\"analyze\" name=\"analyze\" class=\"peer h-4 w-4 rounded accent-indigo-500 mb-2.5\"{analyze}>\
         <label for=\"analyze\" class=\"{NOTE_CLASSES} mb-2\">analyze the run</label>\
         <div class=\"hidden peer-checked:contents\">{}{}</div>\
         </div>\
         </form>",
        agent_fields(
            &registry,
            "",
            selection.agent("", ["", "", DEFAULT_THINKING])
        ),
        select("game", "game", &games, selection.get("game", DEFAULT_GAME)),
        agent_fields(
            &registry,
            crate::serve::ANALYST_PREFIX,
            selection.agent(
                crate::serve::ANALYST_PREFIX,
                [
                    DEFAULT_ANALYST,
                    DEFAULT_ANALYST_MODEL,
                    DEFAULT_ANALYST_THINKING
                ]
            )
        ),
        analyst_seconds_field(selection, crate::serve::ANALYST_PREFIX),
        force = checked(selection.get("force", "") == "on"),
        analyze = checked(selection.get("analyze", "") == "on"),
    ))
}

/// The attribute marking a checkbox checked.
fn checked(on: bool) -> &'static str {
    if on { " checked" } else { "" }
}

/// The form starting an analysis of the run.
fn analysis_panel(name: &str) -> std::io::Result<String> {
    let registry = registry::load()?;

    Ok(format!(
        "<form method=\"post\" action=\"/run/{}/analyze\" class=\"{CARD_CLASSES} p-4 flex flex-wrap items-end gap-4\">\
         {}{}<button class=\"{BUTTON_CLASSES} {CONTROL_HEIGHT}\">analyze</button></form>",
        escape(name),
        agent_fields(
            &registry,
            "",
            [
                DEFAULT_ANALYST,
                DEFAULT_ANALYST_MODEL,
                DEFAULT_ANALYST_THINKING
            ]
        ),
        analyst_seconds_field(&Selection::default(), ""),
    ))
}

/// The field choosing the seconds the analyst under `prefix` is given.
fn analyst_seconds_field(selection: &Selection, prefix: &str) -> String {
    let name = format!("{prefix}seconds");
    let seconds = selection
        .get(&name, "")
        .parse::<u64>()
        .unwrap_or(docker::Analyst::DEFAULT_LIMIT_SECONDS);

    format!(
        "<label class=\"w-24\"><span class=\"{LABEL_CLASSES}\">{}</span>\
         <input class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" type=\"number\" name=\"{name}\" value=\"{seconds}\" min=\"{}\"></label>",
        explained(
            "seconds",
            "the seconds the analyst is given per run, one turn"
        ),
        docker::LAST_CALL_SECONDS,
    )
}

/// The dropdowns choosing an agent, the way one is chosen everywhere: the
/// harness, the model and the thinking level, named under `prefix` in the
/// form, with `selected` marked.
fn agent_fields(registry: &registry::Registry, prefix: &str, selected: [&str; 3]) -> String {
    let harnesses: Vec<&str> = registry
        .harnesses
        .iter()
        .map(|harness| harness.name.as_str())
        .collect();
    let models: Vec<&str> = registry
        .models
        .iter()
        .map(|model| model.name.as_str())
        .collect();
    let mut levels = vec![""];
    levels.extend(registry::THINKING_LEVELS);

    let [harness_field, model_field, thinking_field] = crate::serve::AGENT_FIELDS;
    let [harness, model, thinking] = selected;

    format!(
        "{}{}{}",
        select(
            &format!("{prefix}{harness_field}"),
            harness_field,
            &harnesses,
            harness
        ),
        select(
            &format!("{prefix}{model_field}"),
            model_field,
            &models,
            model
        ),
        select(
            &format!("{prefix}{thinking_field}"),
            thinking_field,
            &levels,
            thinking
        ),
    )
}

/// A dropdown named `name` under `label`, offering `options` with `selected`
/// marked.
///
/// The dropdowns grow to fill the form row, which puts the right edge of the
/// form on the edge the tables end on.
fn select(name: &str, label: &str, options: &[&str], selected: &str) -> String {
    let mut rendered = format!(
        "<label class=\"grow basis-44\"><span class=\"{LABEL_CLASSES}\">{label}</span>\
         <select class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" name=\"{name}\">"
    );
    for option in options {
        let marked = if *option == selected { " selected" } else { "" };
        rendered.push_str(&format!("<option{marked}>{}</option>", escape(option)));
    }
    rendered.push_str("</select></label>");
    rendered
}

/// One run: its state and figures first, the entries, the pushes, the console,
/// then the rest.
pub(crate) fn run_page(name: &str, notice: &Notice) -> std::io::Result<String> {
    let directory = run_directory(name)?;
    let run = runs::read(&directory)?;
    let entry = RunEntry::new(&directory, run, &live_runs(), &tournament::placements()?);

    let mut body = format!(
        "<div data-refresh=\"run\"><div class=\"flex items-center gap-3\">\
         <span class=\"text-lg font-semibold text-neutral-100 {MONO_CLASSES}\">{}</span>{}{}{}</div>",
        escape(name),
        entry.state(),
        entry.analysis_pill(),
        entry.stop_form()
    );
    body.push_str(&notice.render());

    let served = entry
        .metrics
        .as_ref()
        .filter(|metrics| !metrics.served_models.is_empty())
        .map(|metrics| format!("served {}", escape(&metrics.served_models.join(" "))))
        .unwrap_or_default();
    let mut facts = vec![
        tile(
            "game",
            &escape(&entry.run.game),
            &joined(&[&entry.run.game_version, &entry.run.architecture]),
            TILE_TEXT_CLASSES,
        ),
        tile(
            "model",
            &escape(&entry.run.model),
            &served,
            TILE_TEXT_CLASSES,
        ),
        tile(
            "harness",
            &escape(&entry.run.harness),
            &joined(&[
                entry.run.thinking.as_deref().unwrap_or_default(),
                &entry.run.harness_version,
            ]),
            TILE_TEXT_CLASSES,
        ),
        tile(
            "started",
            &format!("{} ago", usage::age(entry.run.started_seconds)),
            &usage::utc_date(entry.run.started_seconds),
            TILE_TEXT_CLASSES,
        ),
    ];
    let turns = ava_game::find(&entry.run.game).map_or(1, |game| game.turns().len());
    if let Some(placement) = &entry.placement {
        facts.push(tile(
            "tournament",
            &tournament_link(&placement.tournament),
            &placement_role(placement, turns),
            TILE_TEXT_CLASSES,
        ));
    }
    if turns > 1 {
        facts.push(tile(
            "turn",
            &format!("{} of {turns}", entry.run.turn + 1),
            &ava_game::find(&entry.run.game)
                .map(|game| runs::turn_task(game, entry.run.turn).to_string())
                .unwrap_or_default(),
            TILE_TEXT_CLASSES,
        ));
    }
    if !entry.run.inputs.is_empty() {
        facts.push(tile(
            "inputs",
            &entry
                .run
                .inputs
                .iter()
                .map(|input| {
                    format!(
                        "<span class=\"block\">{} <span class=\"{MUTED_CLASSES}\">from</span> {}</span>",
                        escape(&input.name),
                        run_link(&input.run)
                    )
                })
                .collect::<String>(),
            "seeded into the workspace",
            TILE_LIST_CLASSES,
        ));
    }
    body.push_str(&tiles(&facts));

    let entry_at = match &entry.record {
        Some(record) => format!("{}s", record.seconds),
        None if entry.live => placeholder(AFTER_THE_RUN),
        None if entry.passed() => placeholder(NOT_KEPT),
        None => placeholder(NO_ENTRY),
    };
    let time = match entry.time_cell() {
        cell if cell.is_empty() => placeholder(UNFINISHED),
        cell => cell,
    };
    let so_far = if entry.live { SO_FAR } else { "" };
    let metric = |value: fn(&ava_wire::Metrics) -> u64| match &entry.metrics {
        Some(metrics) => value(metrics).to_string(),
        None if entry.live => placeholder(AFTER_THE_RUN),
        None => placeholder(NOT_RECORDED),
    };
    let figures = [
        tile("points", &entry.points_tile(), "", TILE_VALUE_CLASSES),
        tile(
            "pushes",
            &entry.attempts.len().to_string(),
            "",
            TILE_VALUE_CLASSES,
        ),
        tile("entry at", &entry_at, "", TILE_VALUE_CLASSES),
        tile("time", &time, "", TILE_VALUE_CLASSES),
        tile(
            "requests",
            &metric(|metrics| metrics.requests),
            so_far,
            TILE_VALUE_CLASSES,
        ),
        tile(
            "output tokens",
            &metric(|metrics| metrics.output_tokens),
            so_far,
            TILE_VALUE_CLASSES,
        ),
    ];
    body.push_str(&tiles(&figures));

    if !entry.live {
        let title = "analysis";
        let record = runs::analysis(&directory)?;
        if entry.analyzing() {
            body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">{title}</p>"));
        } else {
            match record {
            None => body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">{title}</p>")),
            Some(record) => match (&record.error, record.report()) {
                (None, Some(report)) => body.push_str(&format!(
                    "<details open data-fold=\"analysis\"><summary class=\"{COLLAPSIBLE_TITLE_CLASSES}\">{title}</summary>{}</details>",
                    report_card(&record, &report)
                )),
                (error, _) => body.push_str(&format!(
                    "<p class=\"{TITLE_CLASSES}\">{title}</p>\
                     <p class=\"mb-4 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-red-300\">{}</p>",
                    escape(error.as_deref().unwrap_or(NO_REPORT_NOTE))
                )),
            },
        }
        }
        if !entry.analyzing() {
            body.push_str(&analysis_panel(name)?);
        }
    }

    if let Some(game) = ava_game::find(&entry.run.game)
        && !entry.live
    {
        let file = runs::kept_file(game, &entry.run);
        let kept = runs::entries(game, &directory, file)?;
        if !kept.is_empty() {
            let record = entry.record.as_ref().map(|record| record.seconds);
            let rows = kept
                .iter()
                .map(|kept| {
                    vec![
                        kept.seconds.to_string(),
                        kept.bytes.to_string(),
                        kept.points.map(points_meter).unwrap_or_default(),
                        if record == Some(kept.seconds) {
                            pill(PASSED_PILL, false, "entry of record")
                        } else {
                            String::new()
                        },
                        format!(
                            "<a class=\"{LINK_CLASSES}\" href=\"/run/{}/entries/{}/{}\">{}</a>",
                            escape(name),
                            kept.seconds,
                            escape(file),
                            escape(file)
                        ),
                    ]
                })
                .collect();
            body.push_str(&format!(
                "<p class=\"{TITLE_CLASSES}\">{}</p>",
                explained(
                    "entries",
                    "what the passing pushes left, ranked as the game ranks them today"
                )
            ));
            body.push_str(&table(
                &["#SECONDS", "#BYTES", "*POINTS", "", "FILE"],
                rows,
                None,
            ));
        }
    }

    let pushes = attempt_rows(&entry.attempts);
    if !pushes.is_empty() {
        body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">pushes</p>"));
        body.push_str(&table(&["#SECONDS", "STATE", "*REASON"], pushes, None));
    }

    if let Some(metrics) = &entry.metrics {
        body.push_str(&format!(
            "<p class=\"{TITLE_CLASSES}\">metrics <span class=\"{NOTE_CLASSES} font-normal\">{so_far}</span></p>"
        ));
        body.push_str(&object_table(
            &serde_json::to_value(metrics).unwrap_or_default(),
        ));
    }

    if let Ok(tail) = console_tail(&directory.join(docker::AGENT_LOG)) {
        body.push_str(&format!(
            "<p class=\"{TITLE_CLASSES}\">console <span class=\"{NOTE_CLASSES} font-normal\">the last {} bytes</span></p><pre class=\"{CONSOLE_CLASSES}\">{}</pre>",
            tail.len(),
            escape(&strip_ansi(&String::from_utf8_lossy(&tail)))
        ));
    }

    let files = RUN_FILES
        .iter()
        .filter(|file| directory.join(file).exists())
        .map(|file| {
            format!(
                "<a class=\"{LINK_CLASSES}\" href=\"/run/{}/{file}\">{file}</a>",
                escape(name)
            )
        })
        .collect::<String>();
    body.push_str(&format!(
        "<p class=\"{TITLE_CLASSES}\">files</p><p class=\"flex flex-wrap gap-x-4 gap-y-1\">{files}</p>"
    ));

    let mut parameters = serde_json::to_value(&entry.run).unwrap_or_default();
    if let Some(object) = parameters.as_object_mut() {
        for section in RUN_RECORD_SECTIONS {
            object.remove(section);
        }
    }
    body.push_str(&format!(
        "<details class=\"mt-8\" data-fold=\"parameters\"><summary class=\"{SUMMARY_CLASSES}\">parameters</summary><div class=\"mt-3\">{}</div></details>",
        object_table(&parameters)
    ));
    body.push_str("</div>");

    Ok(page(name, &body))
}

/// The best of every played pairing, grouped over the finished runs.
pub(crate) fn scoreboard_page() -> std::io::Result<String> {
    struct Standing {
        runs: u64,
        passed: u64,
        /// The best entry of record, with the seconds it arrived at.
        best: Option<(Option<u64>, u64)>,
    }

    let runs = collect_runs()?;
    let mut standings: Vec<(String, String, String, Standing)> = Vec::new();

    for run in runs.iter().filter(|run| run.run.finished_seconds.is_some()) {
        let key = (game_label(&run.run), run.run.model.clone(), run.agent());

        let standing = match standings
            .iter_mut()
            .find(|(game, model, agent, _)| (game, model, agent) == (&key.0, &key.1, &key.2))
        {
            Some((_, _, _, standing)) => standing,
            None => {
                standings.push((
                    key.0,
                    key.1,
                    key.2,
                    Standing {
                        runs: 0,
                        passed: 0,
                        best: None,
                    },
                ));
                &mut standings.last_mut().expect("just pushed").3
            }
        };

        standing.runs += 1;
        standing.passed += u64::from(run.passed());
        if let Some(record) = &run.record
            && standing
                .best
                .is_none_or(|(points, _)| record.points > points)
        {
            standing.best = Some((record.points, record.seconds));
        }
    }

    standings.sort_by(|left, right| {
        left.0.cmp(&right.0).then(
            right
                .3
                .best
                .map(|(points, _)| points)
                .cmp(&left.3.best.map(|(points, _)| points)),
        )
    });

    let rows = standings
        .iter()
        .map(|(game, model, agent, standing)| {
            vec![
                game.clone(),
                escape(model),
                agent.clone(),
                standing.runs.to_string(),
                standing.passed.to_string(),
                standing
                    .best
                    .and_then(|(points, _)| points.map(points_meter))
                    .unwrap_or_default(),
                standing
                    .best
                    .map(|(_, seconds)| seconds.to_string())
                    .unwrap_or_default(),
            ]
        })
        .collect();

    let body = format!(
        "<p class=\"{FIRST_TITLE_CLASSES}\">{}</p>{}",
        explained(
            "scoreboard",
            "the best entry of every pairing, ranked as the games rank today"
        ),
        table(
            &[
                "GAME", "MODEL", "HARNESS", "#RUNS", "#PASSED", "*BEST", "#SECONDS",
            ],
            rows,
            Some("nothing played yet"),
        )
    );

    Ok(page("scoreboard", &body))
}

/// Every game as a card: the name, its turns and the record on its face,
/// the standings, the heatmap and the task folded behind it.
pub(crate) fn games_page() -> std::io::Result<String> {
    let runs = collect_runs()?;
    let mut cards = String::new();

    for game in games()? {
        let played: Vec<&RunEntry> = runs
            .iter()
            .filter(|run| {
                run.run.game == game && run.run.turn == 0 && run.run.finished_seconds.is_some()
            })
            .collect();
        cards.push_str(&game_card(&game, &played));
    }

    let mut body = format!("<div class=\"{STOREFRONT_CLASSES}\">{cards}</div>");

    if let Ok(instructions) =
        std::fs::read_to_string(std::path::Path::new(GAMES_DIRECTORY).join(INSTRUCTIONS_FILE))
    {
        body.push_str(&format!(
            "<div class=\"{CARD_CLASSES} mt-8 px-4 pb-4\">{}</div>",
            ava_markdown::render(&instructions)
        ));
    }

    Ok(page("games", &body))
}

/// The card of one game over the finished runs that `played` it, folding
/// out to the text of its task.
fn game_card(game: &str, played: &[&RunEntry]) -> String {
    let turns = ava_game::find(game).map_or(1, |found| found.turns().len());
    let tasks: Vec<String> = (0..turns)
        .map(|turn| {
            std::fs::read_to_string(docker::task_directory(game, turn).join(TASK_FILE))
                .unwrap_or_default()
        })
        .collect();
    let task = tasks.first().cloned().unwrap_or_default();
    let passed = played.iter().filter(|run| run.passed()).count() as u64;
    let best = played
        .iter()
        .filter_map(|run| run.record.as_ref().map(|record| (*run, record)))
        .max_by_key(|(_, record)| (record.points, record.seconds));

    format!(
        "<details class=\"{GAME_CARD_CLASSES}\">{}\
         <div class=\"{GAME_BODY_CLASSES}\">{}</div></details>",
        game_face(game, &task, played.len() as u64, passed, best),
        tasks
            .iter()
            .filter(|text| !text.is_empty())
            .map(|text| ava_markdown::render(text))
            .collect::<Vec<String>>()
            .join(TASK_SEPARATOR)
    )
}

/// The face of a card: the name and its turns, the title of the task and
/// the image it plays on, then the cover beside the runs and the record.
fn game_face(
    game: &str,
    task: &str,
    runs: u64,
    passed: u64,
    best: Option<(&RunEntry, &runs::Entry)>,
) -> String {
    let image = ava_game::find(game)
        .and_then(|found| found.image())
        .map(|image| {
            format!(
                "<span class=\"{MUTED_CLASSES} {MONO_CLASSES} text-xs whitespace-nowrap\">image {}</span>",
                escape(image)
            )
        })
        .unwrap_or_default();

    let (record, holder) = match best {
        Some((run, entry)) => match entry.points {
            Some(points) => (
                points_meter(points),
                format!("{} \u{00b7} {}", escape(&run.run.model), run.agent()),
            ),
            None => (placeholder(UNRANKED), String::new()),
        },
        None if passed > 0 => (placeholder(NOT_KEPT), String::new()),
        None => (placeholder(NO_ENTRY), String::new()),
    };

    format!(
        "<summary class=\"{GAME_FACE_CLASSES}\">\
         <span class=\"flex items-center gap-3\">\
         <span class=\"{GAME_NAME_CLASSES}\">{}</span>{}<span class=\"flex-1\"></span>\
         {}\
         </span>\
         <span class=\"{GAME_TAGLINE_CLASSES}\"><span class=\"{NOTE_CLASSES} truncate\">{}</span>{image}</span>\
         <span class=\"flex items-start gap-4 mt-4\">{}\
         <span class=\"{FACTS_CLASSES}\">{}{}<span class=\"{HOLDER_CLASSES}\">{holder}</span></span>\
         </span>\
         </summary>",
        escape(game),
        turn_badges(game),
        chevron(CHEVRON_CLASSES),
        escape(task_title(task)),
        cover(game, best),
        fact(
            "runs",
            &format!("<span class=\"{FACT_VALUE_CLASSES}\">{runs}</span>")
        ),
        fact("record", &record),
    )
}

/// The cover of a card: the image the game folder provides, else the entry
/// of record, else an empty frame.
fn cover(game: &str, best: Option<(&RunEntry, &runs::Entry)>) -> String {
    if cover_path(game).is_some() {
        return format!(
            "<span class=\"{COVER_CLASSES}\"><img class=\"{COVER_IMAGE_CLASSES}\" src=\"/games/{}/cover\" alt=\"\"></span>",
            escape(game)
        );
    }

    let Some((run, entry)) = best else {
        return format!("<span class=\"{COVER_EMPTY_CLASSES}\"></span>");
    };

    let head = read_head(&entry.path, COVER_BYTES).unwrap_or_default();
    let art = if entry.bytes <= TEXT_ENTRY_LIMIT && is_text(&head) {
        text_cover(&head)
    } else {
        byte_cover(&head)
    };

    format!(
        "<span class=\"{COVER_CLASSES}\" title=\"{}\">{art}</span>",
        escape(&format!(
            "the entry of record, {} bytes, kept by {}",
            entry.bytes, run.name
        ))
    )
}

/// The cover image of the game `name` in its folder, with its content type,
/// for a name the games directory knows.
fn cover_path(name: &str) -> Option<(std::path::PathBuf, &'static str)> {
    if !games().ok()?.iter().any(|known| known == name) {
        return None;
    }

    COVER_FILES.iter().find_map(|(file, content_type)| {
        let path = std::path::Path::new(GAMES_DIRECTORY).join(name).join(file);
        path.is_file().then_some((path, *content_type))
    })
}

/// The cover image of the game `name` with its content type, if it has one.
pub(crate) fn game_cover(name: &str) -> Option<(Vec<u8>, &'static str)> {
    let (path, content_type) = cover_path(name)?;
    Some((std::fs::read(path).ok()?, content_type))
}

/// The first `limit` bytes of the file at `path`.
fn read_head(path: &std::path::Path, limit: usize) -> std::io::Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    let mut head = Vec::with_capacity(limit);
    std::io::Read::read_to_end(&mut std::io::Read::take(file, limit as u64), &mut head)?;

    Ok(head)
}

/// Whether `bytes` are printable ASCII and whitespace throughout.
fn is_text(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_graphic() || byte.is_ascii_whitespace())
}

/// A text entry as its text, one line centered and more read from the top.
fn text_cover(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let align = if text.trim().lines().count() <= 1 {
        "self-center text-center"
    } else {
        "self-start"
    };

    format!(
        "<span class=\"{COVER_TEXT_CLASSES} {align}\">{}</span>",
        escape(text.trim_end())
    )
}

/// A binary entry as a grid of its first bytes, one cell each, shaded by
/// value: a zero byte leaves the surface bare and the cells past the end of
/// a short file stay empty, so the size of the file is part of the picture.
fn byte_cover(bytes: &[u8]) -> String {
    let mut cells = String::new();
    for (index, byte) in bytes.iter().enumerate().filter(|(_, byte)| **byte != 0) {
        cells.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"1\" height=\"1\" fill-opacity=\"{:.2}\"/>",
            index % COVER_SIDE,
            index / COVER_SIDE,
            f64::from(*byte) / f64::from(u8::MAX)
        ));
    }

    format!(
        "<svg class=\"{COVER_ART_CLASSES}\" viewBox=\"0 0 {COVER_SIDE} {COVER_SIDE}\" fill=\"currentColor\" shape-rendering=\"crispEdges\">{cells}</svg>"
    )
}

/// A chevron pointing down, turned by `classes` where it marks an open fold.
fn chevron(classes: &str) -> String {
    format!(
        "<svg class=\"{classes}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.75\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M6 9l6 6 6-6\"/></svg>"
    )
}

/// The first heading of a task, or nothing when it has none.
fn task_title(task: &str) -> &str {
    task.lines()
        .find_map(|line| line.strip_prefix("# "))
        .unwrap_or_default()
        .trim()
}

/// One figure on the face of a card, its label beside it.
fn fact(label: &str, value: &str) -> String {
    format!(
        "<span class=\"{FACT_ROW_CLASSES}\"><span class=\"{FACT_LABEL_CLASSES}\">{label}</span><span class=\"flex-1 min-w-0\">{value}</span></span>"
    )
}

/// The turns of a game with several, one pill per task, nothing for a game
/// with one turn.
fn turn_badges(game: &str) -> String {
    let Some(found) = ava_game::find(game) else {
        return String::new();
    };
    if found.turns().len() < 2 {
        return String::new();
    }

    found
        .turns()
        .iter()
        .map(|turn| pill(TURN_PILL, false, &escape(turn.task)))
        .collect()
}

/// The tournaments: the form opening one, and every tournament on disk.
pub(crate) fn tournaments_page(notice: &Notice, selection: &Selection) -> std::io::Result<String> {
    let games: Vec<&str> = ava_game::GAMES.iter().map(|game| game.name()).collect();
    let limit = selection
        .get("limit", "")
        .parse::<u64>()
        .unwrap_or(docker::Agent::DEFAULT_LIMIT_SECONDS);
    let last_call = docker::LAST_CALL_SECONDS;
    let combats = selection
        .get("combats", "")
        .parse::<u64>()
        .unwrap_or(tournament::DEFAULT_COMBATS);

    let mut body = format!(
        "<p class=\"{FIRST_TITLE_CLASSES}\">new tournament</p>\
         <form method=\"post\" action=\"/tournaments/create\" class=\"{CARD_CLASSES} p-4 flex flex-wrap items-end gap-4\">\
         <label class=\"grow basis-44\"><span class=\"{LABEL_CLASSES}\">name</span>\
         <input class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" type=\"text\" name=\"name\" value=\"{}\" placeholder=\"letters, digits, dashes\" required></label>\
         {}\
         <label class=\"w-24\"><span class=\"{LABEL_CLASSES}\">{}</span>\
         <input class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" type=\"number\" name=\"limit\" value=\"{limit}\" min=\"{last_call}\"></label>\
         <label class=\"w-24\"><span class=\"{LABEL_CLASSES}\">{}</span>\
         <input class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" type=\"number\" name=\"combats\" value=\"{combats}\" min=\"1\"></label>\
         <button class=\"{BUTTON_CLASSES} {CONTROL_HEIGHT}\">open</button>\
         <div class=\"w-full flex flex-wrap items-end gap-4\">\
         <input type=\"checkbox\" id=\"analyze\" name=\"analyze\" class=\"peer h-4 w-4 rounded accent-indigo-500 mb-2.5\"{analyze}>\
         <label for=\"analyze\" class=\"{NOTE_CLASSES} mb-2\">analyze every run of a round</label>\
         <div class=\"hidden peer-checked:contents\">{}{}</div>\
         </div>\
         </form>",
        escape(selection.get("name", "")),
        select(
            "game",
            "game",
            &games,
            selection.get("game", games.first().copied().unwrap_or_default())
        ),
        explained(
            "seconds",
            &format!("the budget of every run, the {last_call} second last call included"),
        ),
        explained(
            "combats",
            "the combats every fight between two entries plays, each best of three rounds",
        ),
        agent_fields(
            &registry::load()?,
            crate::serve::ANALYST_PREFIX,
            selection.agent(
                crate::serve::ANALYST_PREFIX,
                [
                    DEFAULT_ANALYST,
                    DEFAULT_ANALYST_MODEL,
                    DEFAULT_ANALYST_THINKING
                ]
            )
        ),
        analyst_seconds_field(selection, crate::serve::ANALYST_PREFIX),
        analyze = checked(selection.get("analyze", "") == "on"),
    );

    let tournaments = tournament::list()?;
    let rows = tournaments
        .iter()
        .map(|record| {
            vec![
                format!(
                    "<a class=\"{LINK_CLASSES}\" href=\"/tournament/{name}\">{name}</a><div class=\"text-xs {MUTED_CLASSES} mt-0.5\">opened {} ago</div>",
                    usage::age(record.created_seconds),
                    name = escape(&record.name)
                ),
                tournament_state(record),
                escape(&record.game),
                record.seats.len().to_string(),
                record.rounds.len().to_string(),
                format!("{}s", record.limit_seconds),
                record.combats.to_string(),
            ]
        })
        .collect();

    body.push_str("<div data-refresh=\"tournaments\">");
    body.push_str(&notice.render());
    body.push_str(&format!(
        "<p class=\"{TITLE_CLASSES}\">tournaments <span class=\"{NOTE_CLASSES} font-normal\">{} on disk</span></p>{}",
        tournaments.len(),
        table(
            &["NAME", "STATE", "GAME", "#SEATS", "#ROUNDS", "*SECONDS", "#COMBATS"],
            rows,
            Some(NO_TOURNAMENTS_NOTE),
        )
    ));
    body.push_str("</div>");

    Ok(page("tournaments", &body))
}

/// The state of a tournament as a pill: playing, open, or how far it got.
fn tournament_state(record: &ava_wire::Tournament) -> String {
    if tournament::playing(&record.name) {
        return pill(
            LIVE_PILL,
            true,
            &format!("playing round {}", record.rounds.len()),
        );
    }

    match record.rounds.last() {
        None => pill(NEUTRAL_PILL, false, "open"),
        Some(round) if round.finished_seconds.is_none() => pill(
            BROKEN_PILL,
            false,
            &format!("round {} broke off", record.rounds.len()),
        ),
        Some(_) => pill(
            NEUTRAL_PILL,
            false,
            &format!("{} rounds played", record.rounds.len()),
        ),
    }
}

/// One tournament: its lobby, its standings and every round it played.
pub(crate) fn tournament_page(
    name: &str,
    notice: &Notice,
    selection: &Selection,
) -> std::io::Result<String> {
    let record = tournament::load(name)?;
    let playing = tournament::playing(name);
    let running = live_runs();
    let registry = registry::load()?;
    let game = ava_game::find(&record.game);

    let play_form = if playing || record.seats.is_empty() {
        String::new()
    } else {
        format!(
            "<form method=\"post\" action=\"/tournament/{}/play\" class=\"flex items-end gap-3\">\
             <label class=\"w-24\"><span class=\"{LABEL_CLASSES}\">{}</span>\
             <input class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" type=\"number\" name=\"parallel\" min=\"1\" placeholder=\"all\"></label>\
             <button class=\"{BUTTON_CLASSES} {CONTROL_HEIGHT}\">play round {}</button></form>",
            escape(name),
            explained(
                "parallel",
                "the most runs the round starts at once, every run of a phase at once when empty"
            ),
            record.rounds.len() + 1
        )
    };

    // The forms stay outside the refreshed regions, so what is chosen in them
    // survives the refresh.
    let mut body = format!(
        "<div class=\"flex items-center gap-3\">\
         <span class=\"text-lg font-semibold text-neutral-100 {MONO_CLASSES}\">{}</span><span data-refresh=\"state\">{}</span><span class=\"grow\"></span>{play_form}</div>",
        escape(name),
        tournament_state(&record),
    );
    body.push_str(&notice.render());
    body.push_str(&format!(
        "<p class=\"{NOTE_CLASSES} mt-1.5\">{} {} \u{00b7} {} \u{00b7} {}s a run \u{00b7} {} combats a fight{} \u{00b7} opened {} ago</p>",
        escape(&record.game),
        version_label(&record.game_version),
        escape(&record.pairing),
        record.limit_seconds,
        record.combats,
        record
            .analyst
            .as_ref()
            .map(|analyst| format!(
                " \u{00b7} analyzed by {} in {}s",
                escape(&analyst.label()),
                record.analyst_seconds
            ))
            .unwrap_or_default(),
        usage::age(record.created_seconds),
    ));

    // The lobby.
    let removable = !record.played() && !playing;
    let seat_rows: Vec<Vec<String>> = record
        .seats
        .iter()
        .enumerate()
        .map(|(seat, agent)| {
            let played = record
                .rounds
                .iter()
                .filter(|round| round.entries.iter().any(|entry| entry.seat == seat))
                .count();
            vec![
                (seat + 1).to_string(),
                agent_label(&agent.harness, agent.thinking.as_deref().unwrap_or("")),
                escape(&agent.model),
                played.to_string(),
                if removable {
                    format!(
                        "<form method=\"post\" action=\"/tournament/{}/unseat\"><input type=\"hidden\" name=\"seat\" value=\"{seat}\"><button class=\"{STOP_CLASSES}\">remove</button></form>",
                        escape(name)
                    )
                } else {
                    String::new()
                },
            ]
        })
        .collect();
    body.push_str(&format!(
        "<div data-refresh=\"lobby\"><p class=\"{TITLE_CLASSES}\">{}</p>{}</div>",
        explained(
            "lobby",
            "the seats of the tournament, joining between rounds and fixed once a round was played"
        ),
        table(
            &["#SEAT", "HARNESS", "*MODEL", "#ROUNDS", ""],
            seat_rows,
            Some(NO_SEATS_NOTE),
        )
    ));
    if removable {
        body.push_str(&format!(
            "<form method=\"post\" action=\"/tournament/{}/seat\" class=\"{CARD_CLASSES} border-t-0 rounded-t-none p-4 flex flex-wrap items-end gap-4\">\
             {}<button class=\"{BUTTON_CLASSES} {CONTROL_HEIGHT}\">seat</button></form>",
            escape(name),
            agent_fields(&registry, "", selection.agent("", ["", "", DEFAULT_THINKING])),
        ));
    }

    body.push_str("<div data-refresh=\"rounds\">");

    // The standings.
    let standings = standings(&record)?;
    if !standings.is_empty() {
        let rows = standings
            .iter()
            .map(|standing| {
                vec![
                    escape(&standing.agent),
                    standing.seats.to_string(),
                    tally_label(&standing.fights),
                    standing
                        .rounds
                        .score()
                        .map(|score| format!("{score:.2}"))
                        .unwrap_or_default(),
                    rating_label(standing.elo),
                    rating_label(standing.bradley_terry),
                ]
            })
            .collect();
        body.push_str(&format!(
            "<p class=\"{TITLE_CLASSES}\">{}</p>{}",
            explained(
                "standings",
                "derived from the matches of the finished rounds between different agents, ordered by Bradley-Terry"
            ),
            table(
                &[
                    "*AGENT",
                    "#SEATS|the seats the agent holds, two seats of one agent count as one entry here",
                    "#FIGHTS|the fights against another agent as won-drawn-lost, a fight with more rounds won than lost is won",
                    "#SCORE|the share of the rounds of those fights won, half for a draw, what the ratings are fed",
                    "#ELO|updated in match order, anchored at 1000",
                    "#BRADLEY-TERRY|fitted over the whole history, anchored at 1000",
                ],
                rows,
                None,
            )
        ));
    }

    // The rounds, newest first.
    for (index, round) in record.rounds.iter().enumerate().rev() {
        let number = index + 1;
        let state = if round.finished_seconds.is_some() {
            format!(
                "finished {} ago",
                usage::age(round.finished_seconds.unwrap_or_default())
            )
        } else if playing && index + 1 == record.rounds.len() {
            "playing".to_string()
        } else {
            "broke off".to_string()
        };
        body.push_str(&format!(
            "<p class=\"{TITLE_CLASSES}\">round {number} <span class=\"{NOTE_CLASSES} font-normal\">started {} ago, {state}</span></p>",
            usage::age(round.started_seconds)
        ));

        let live = playing && index + 1 == record.rounds.len();
        body.push_str(&round_graph(&record, round, game, &running, live));

        let pairings = tournament::pairings(&record, round)?;
        if !pairings.is_empty() {
            // The attacks of a record from before the turns pair every seat
            // with every other in both directions.
            let ordered = pairings.iter().any(|pairing| {
                pairings
                    .iter()
                    .any(|other| (other.first, other.second) == (pairing.second, pairing.first))
            });
            body.push_str(&format!(
                "<div class=\"mt-4\">{}</div>",
                cross_table(&record, round, &pairings, ordered, live)
            ));
        }
    }

    body.push_str(&format!(
        "<p class=\"{TITLE_CLASSES}\">files</p><p class=\"flex flex-wrap gap-x-4 gap-y-1\">{}</p>",
        tournament_files(name)
            .iter()
            .map(|file| {
                format!(
                    "<a class=\"{LINK_CLASSES}\" href=\"/tournament/{}/{file}\">{file}</a>",
                    escape(name)
                )
            })
            .collect::<String>()
    ));
    body.push_str("</div>");

    Ok(page(name, &body))
}

/// The runs of a round as the graph the tournament walked: a column per turn,
/// a row per seat, every run a node linking its page with its state, and an
/// edge from every entry a run got as its input to that run. While the round
/// is `live`, a run not started yet shows as queued.
fn round_graph(
    record: &ava_wire::Tournament,
    round: &ava_wire::Round,
    game: Option<&dyn ava_game::Game>,
    running: &[String],
    live: bool,
) -> String {
    struct Node {
        seat: usize,
        turn: usize,
        run: String,
        record: Option<ava_wire::Run>,
        points: Option<u64>,
        x: f64,
        y: f64,
    }

    let mut nodes: Vec<Node> = Vec::new();
    let mut place = |seat: usize, turn: usize, run: &str, attempt: Option<u64>| {
        if nodes.iter().any(|node| node.run == run) {
            return;
        }
        let directory = std::path::Path::new(docker::RUN_DIRECTORY).join(run);
        let record = runs::read(&directory).ok();
        let points = match (game, attempt) {
            (Some(game), Some(attempt)) => {
                runs::entries(game, &directory, runs::turn_entry(game, turn))
                    .ok()
                    .and_then(|kept| kept.into_iter().find(|kept| kept.seconds == attempt))
                    .and_then(|kept| kept.points)
            }
            _ => None,
        };
        nodes.push(Node {
            seat,
            turn,
            run: run.to_string(),
            record,
            points,
            x: 0.0,
            y: 0.0,
        });
    };
    for entry in &round.entries {
        place(entry.seat, entry.turn, &entry.run, entry.attempt);
    }
    // The attacks of a record from before the turns played the second turn.
    for pairing in &round.pairings {
        if let Some(run) = &pairing.run {
            place(pairing.first, LEGACY_ATTACK_TURN, run, None);
        }
    }

    let seats = record.seats.len();
    let turns = game
        .map_or(1, |game| game.turns().len())
        .max(nodes.iter().map(|node| node.turn + 1).max().unwrap_or(1));

    // A seat's row is as tall as its fullest column, so nodes never overlap
    // when a turn holds several runs of one seat.
    let mut stacked: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();
    let mut rows = vec![1usize; seats];
    for node in &nodes {
        let count = stacked.entry((node.seat, node.turn)).or_default();
        *count += 1;
        if node.seat < seats {
            rows[node.seat] = rows[node.seat].max(*count);
        }
    }
    let row_height =
        |stack: usize| stack as f64 * GRAPH_NODE_HEIGHT + (stack as f64 - 1.0) * GRAPH_ROW_GAP;
    let mut row_top = Vec::with_capacity(seats);
    let mut y = GRAPH_HEADER_HEIGHT;
    for stack in &rows {
        row_top.push(y);
        y += row_height(*stack) + GRAPH_ROW_GAP;
    }
    let height = y - GRAPH_ROW_GAP + GRAPH_PADDING;
    let width = turns as f64 * GRAPH_NODE_WIDTH + (turns as f64 - 1.0) * GRAPH_COLUMN_GAP;

    let mut filled: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();
    for node in &mut nodes {
        let slot = filled.entry((node.seat, node.turn)).or_default();
        node.x = node.turn as f64 * (GRAPH_NODE_WIDTH + GRAPH_COLUMN_GAP);
        node.y = row_top
            .get(node.seat)
            .copied()
            .unwrap_or(GRAPH_HEADER_HEIGHT)
            + *slot as f64 * (GRAPH_NODE_HEIGHT + GRAPH_ROW_GAP);
        *slot += 1;
    }

    let mut svg = format!(
        "<svg class=\"block w-full {GRAPH_CLASSES}\" style=\"max-width:{width}px\" viewBox=\"0 0 {width} {height}\" font-size=\"{GRAPH_FONT_SIZE}\">"
    );

    for turn in 0..turns {
        let task = game
            .and_then(|game| game.turns().get(turn))
            .map(|turn| turn.task.to_string())
            .unwrap_or_else(|| format!("turn {}", turn + 1));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" class=\"{GRAPH_HEADER_TEXT_CLASSES}\">{}</text>",
            turn as f64 * (GRAPH_NODE_WIDTH + GRAPH_COLUMN_GAP),
            GRAPH_HEADER_HEIGHT - GRAPH_ROW_GAP,
            escape(&task)
        ));
    }

    for node in &nodes {
        let Some(run) = &node.record else {
            continue;
        };
        for input in &run.inputs {
            let Some(source) = nodes.iter().find(|source| source.run == input.run) else {
                continue;
            };
            let (from_x, from_y) = (
                source.x + GRAPH_NODE_WIDTH,
                source.y + GRAPH_NODE_HEIGHT / 2.0,
            );
            let (to_x, to_y) = (node.x, node.y + GRAPH_NODE_HEIGHT / 2.0);
            let bend = (from_x + to_x) / 2.0;
            svg.push_str(&format!(
                "<path d=\"M{from_x} {from_y} C{bend} {from_y} {bend} {to_y} {to_x} {to_y}\" class=\"{GRAPH_EDGE_CLASSES}\" fill=\"none\"><title>{}</title></path>",
                escape(&input.name)
            ));
        }
    }

    for node in &nodes {
        let live_run = running.contains(&docker::scorer_container(&node.run));
        let (state, tint, pulsing) = match (&node.record, live_run) {
            (_, true) => ("live", LIVE_PILL, true),
            (Some(run), false) if run.passed() => ("passed", PASSED_PILL, false),
            (Some(run), false) if run.finished_seconds.is_some() => ("failed", FAILED_PILL, false),
            (Some(_), false) => ("unfinished", BROKEN_PILL, false),
            (None, false) if live => ("queued", STARTING_PILL, true),
            (None, false) => ("missing", BROKEN_PILL, false),
        };
        let agent = record
            .seats
            .get(node.seat)
            .map(|agent| format!("{} on {}", agent.harness, agent.model))
            .unwrap_or_default();
        let label = format!("{} \u{00b7} {agent}", node.seat + 1);
        let shown = if label.chars().count() > GRAPH_LABEL_CHARS {
            format!(
                "{}\u{2026}",
                label
                    .chars()
                    .take(GRAPH_LABEL_CHARS - 1)
                    .collect::<String>()
            )
        } else {
            label.clone()
        };
        let points = node
            .points
            .map(|points| {
                format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" class=\"{GRAPH_POINTS_CLASSES}\">{points}</text>",
                    node.x + GRAPH_NODE_WIDTH - GRAPH_TEXT_INSET,
                    node.y + GRAPH_LINE_ONE
                )
            })
            .unwrap_or_default();
        let dot_class = if pulsing { "animate-pulse" } else { "" };
        svg.push_str(&format!(
            "<a href=\"/run/{run}\"><title>{title}</title>\
             <rect x=\"{x}\" y=\"{y}\" width=\"{GRAPH_NODE_WIDTH}\" height=\"{GRAPH_NODE_HEIGHT}\" rx=\"6\" class=\"{GRAPH_NODE_CLASSES}\"/>\
             <text x=\"{text_x}\" y=\"{line_one}\" class=\"{GRAPH_LABEL_CLASSES}\">{shown}</text>{points}\
             <text x=\"{text_x}\" y=\"{line_two}\" class=\"{GRAPH_RUN_CLASSES}\">{run}</text>\
             <circle cx=\"{dot_x}\" cy=\"{dot_y}\" r=\"3\" fill=\"currentColor\" class=\"{tint} {dot_class}\"/>\
             <text x=\"{state_x}\" y=\"{line_two}\" text-anchor=\"end\" fill=\"currentColor\" class=\"{GRAPH_STATE_CLASSES} {tint}\">{state}</text>\
             </a>",
            run = escape(&node.run),
            title = escape(&format!("{label}, {state}")),
            x = node.x,
            y = node.y,
            text_x = node.x + GRAPH_TEXT_INSET,
            line_one = node.y + GRAPH_LINE_ONE,
            line_two = node.y + GRAPH_LINE_TWO,
            dot_x = node.x + GRAPH_NODE_WIDTH - GRAPH_TEXT_INSET - GRAPH_STATE_WIDTH,
            dot_y = node.y + GRAPH_LINE_TWO - GRAPH_DOT_LIFT,
            state_x = node.x + GRAPH_NODE_WIDTH - GRAPH_TEXT_INSET,
        ));
    }

    svg.push_str("</svg>");
    format!("<div class=\"{CARD_CLASSES} p-4 overflow-x-auto\">{svg}</div>")
}

/// The place of one agent on the leaderboard of a tournament.
struct Standing {
    agent: String,
    seats: usize,
    /// The fights against another agent by outcome, from the agent's view: a
    /// fight with more rounds won than lost is won.
    fights: ava_wire::Tally,
    /// The rounds across those fights, from the agent's view, whose share
    /// won is the score.
    rounds: ava_wire::Tally,
    elo: Option<f64>,
    bradley_terry: Option<f64>,
}

/// The standings of a tournament, every agent its seats hold, rated over the
/// matches of the finished rounds, Bradley-Terry first.
fn standings(record: &ava_wire::Tournament) -> std::io::Result<Vec<Standing>> {
    let matches = tournament::matches(record)?;
    let mut pairings = Vec::new();
    for round in record.finished_rounds() {
        pairings.extend(tournament::pairings(record, round)?);
    }
    let elo = ava_game::scoring::Elo.leaderboard(&matches);
    let bradley_terry = ava_game::scoring::BradleyTerry.leaderboard(&matches);
    let rating = |leaderboard: &[ava_game::scoring::Rating], agent: &str| {
        leaderboard
            .iter()
            .find(|rating| rating.agent == agent)
            .map(|rating| rating.rating)
    };

    let mut agents: Vec<String> = Vec::new();
    for seat in &record.seats {
        let label = seat.label();
        if !agents.contains(&label) {
            agents.push(label);
        }
    }

    let mut standings: Vec<Standing> = agents
        .into_iter()
        .map(|agent| {
            let mut fights = ava_wire::Tally::default();
            let mut rounds = ava_wire::Tally::default();
            for pairing in &pairings {
                let first = record.seats.get(pairing.first).map(ava_wire::Agent::label);
                let second = record.seats.get(pairing.second).map(ava_wire::Agent::label);
                if first == second || pairing.tally.rounds() == 0 {
                    continue;
                }
                let view = if first.as_deref() == Some(&agent) {
                    pairing.tally
                } else if second.as_deref() == Some(&agent) {
                    ava_wire::Tally {
                        won: pairing.tally.lost,
                        drawn: pairing.tally.drawn,
                        lost: pairing.tally.won,
                    }
                } else {
                    continue;
                };
                rounds.won += view.won;
                rounds.drawn += view.drawn;
                rounds.lost += view.lost;
                match view.won.cmp(&view.lost) {
                    std::cmp::Ordering::Greater => fights.won += 1,
                    std::cmp::Ordering::Equal => fights.drawn += 1,
                    std::cmp::Ordering::Less => fights.lost += 1,
                }
            }

            Standing {
                seats: record
                    .seats
                    .iter()
                    .filter(|seat| seat.label() == agent)
                    .count(),
                fights,
                rounds,
                elo: rating(&elo, &agent),
                bradley_terry: rating(&bradley_terry, &agent),
                agent,
            }
        })
        .collect();

    standings.sort_by(|left, right| {
        right
            .bradley_terry
            .partial_cmp(&left.bradley_terry)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.agent.cmp(&right.agent))
    });

    Ok(standings)
}

/// A rating rounded to the point, or nothing for an agent without matches.
/// A tally as won-drawn-lost.
fn tally_label(tally: &ava_wire::Tally) -> String {
    format!("{}-{}-{}", tally.won, tally.drawn, tally.lost)
}

fn rating_label(rating: Option<f64>) -> String {
    rating
        .map(|rating| format!("{}", rating.round() as i64))
        .unwrap_or_default()
}

/// The `pairings` of one round as a cross table: the tally of the row's seat
/// against the column's seat, and its total across the row. An `ordered`
/// round pairs every seat with every other twice, once attacking and once
/// defending, so the row is the attacker and nothing is mirrored. While the
/// round is `live`, a pairing without rounds is an attack still going.
fn cross_table(
    record: &ava_wire::Tournament,
    round: &ava_wire::Round,
    pairings: &[ava_wire::Pairing],
    ordered: bool,
    live: bool,
) -> String {
    let seats = round.entries.len();
    let mut headers: Vec<String> = vec![if ordered {
        "*SEAT|the seat of the row attacks the entry of the column".to_string()
    } else {
        "*SEAT".to_string()
    }];
    headers.extend((1..=seats).map(|seat| format!("#{seat}")));
    headers.push(
        "#TOTAL|fights won, drawn and lost across the row, forfeits included, pairings without a fight left out"
            .to_string(),
    );
    let headers: Vec<&str> = headers.iter().map(String::as_str).collect();

    let rows = (0..seats)
        .map(|row| {
            let mut total = ava_wire::Tally::default();
            let mut cells = vec![format!(
                "{} {}",
                row + 1,
                record
                    .seats
                    .get(row)
                    .map(|agent| format!(
                        "<span class=\"{MUTED_CLASSES}\">{}</span>",
                        escape(&agent.label())
                    ))
                    .unwrap_or_default()
            )];

            for column in 0..seats {
                if row == column {
                    cells.push(format!("<span class=\"{MUTED_CLASSES}\">\u{00b7}</span>"));
                    continue;
                }

                let fought = pairings.iter().find_map(|pairing| {
                    if pairing.first == row && pairing.second == column {
                        Some((
                            pairing.tally,
                            pairing.reason.as_deref(),
                            pairing.run.as_deref(),
                        ))
                    } else if !ordered && pairing.first == column && pairing.second == row {
                        Some((
                            ava_wire::Tally {
                                won: pairing.tally.lost,
                                drawn: pairing.tally.drawn,
                                lost: pairing.tally.won,
                            },
                            pairing.reason.as_deref(),
                            pairing.run.as_deref(),
                        ))
                    } else {
                        None
                    }
                });

                match fought {
                    Some((tally, None, Some(run))) if live && tally.rounds() == 0 => {
                        let started = std::path::Path::new(docker::RUN_DIRECTORY)
                            .join(run)
                            .join(docker::RUN_FILE)
                            .is_file();
                        cells.push(played(
                            &if started {
                                pill(LIVE_PILL, true, "live")
                            } else {
                                pill(STARTING_PILL, true, "queued")
                            },
                            Some(run),
                        ));
                    }
                    Some((tally, reason, run)) => {
                        match tally.won.cmp(&tally.lost) {
                            _ if tally.rounds() == 0 => {}
                            std::cmp::Ordering::Greater => total.won += 1,
                            std::cmp::Ordering::Equal => total.drawn += 1,
                            std::cmp::Ordering::Less => total.lost += 1,
                        }
                        cells.push(pairing_cell(&tally, reason, run));
                    }
                    None => cells.push(String::new()),
                }
            }

            cells.push(format!(
                "<span class=\"{MONO_CLASSES} {}\">{}</span>",
                tint(&total),
                tally_label(&total)
            ));
            cells
        })
        .collect();

    format!(
        "<p class=\"{NOTE_CLASSES} mb-2\">{}</p>{}",
        round_summary(round, pairings),
        table(&headers, rows, None)
    )
}

/// One line on what a round came to: how many of its runs left an entry and
/// what became of the pairings.
fn round_summary(round: &ava_wire::Round, pairings: &[ava_wire::Pairing]) -> String {
    let entries = round
        .entries
        .iter()
        .filter(|entry| entry.attempt.is_some())
        .count();
    let mut fought = 0;
    let mut forfeited = 0;
    let mut unplayed = 0;
    let mut playing = 0;
    for pairing in pairings {
        match (pairing.tally.rounds(), &pairing.reason, &pairing.run) {
            (0, None, Some(_)) => playing += 1,
            (0, _, _) => unplayed += 1,
            (_, Some(_), None) => forfeited += 1,
            _ => fought += 1,
        }
    }

    let mut parts = vec![format!(
        "{entries} of {} runs left an entry",
        round.entries.len()
    )];
    for (count, what) in [
        (fought, "fought"),
        (forfeited, "forfeited"),
        (unplayed, "without a fight"),
        (playing, "playing"),
    ] {
        if count > 0 {
            parts.push(format!("{count} {what}"));
        }
    }

    parts.join(" \u{00b7} ")
}

/// The colour of a tally from the view of its first side.
fn tint(tally: &ava_wire::Tally) -> &'static str {
    match tally.won.cmp(&tally.lost) {
        std::cmp::Ordering::Greater => AHEAD_CLASSES,
        std::cmp::Ordering::Less => BEHIND_CLASSES,
        std::cmp::Ordering::Equal => LEVEL_CLASSES,
    }
}

/// A tally as `won-drawn-lost`, tinted by who came out ahead, with the reason
/// behind it as a tooltip when there is one and the run that played it linked.
/// One pairing of the cross table, from the view of the row: the tally of a
/// fight with its run, `forfeit` tinted by who took it, or `none` for a
/// pairing that saw no fight, the reason behind the hover either way.
fn pairing_cell(tally: &ava_wire::Tally, reason: Option<&str>, run: Option<&str>) -> String {
    let reason = reason.unwrap_or_default();
    if tally.rounds() == 0 {
        return played(&format!("<span class=\"{MUTED_CLASSES}\">none</span>"), run);
    }

    let label = if run.is_none() && !reason.is_empty() {
        format!("<span class=\"{}\">forfeit</span>", tint(tally))
    } else {
        format!(
            "<span class=\"{MONO_CLASSES} {}\">{}</span>",
            tint(tally),
            tally_label(tally)
        )
    };

    played(&explained(&label, reason), run)
}

/// `label`, linking to the run that played the pairing when one did.
fn played(label: &str, run: Option<&str>) -> String {
    match run {
        Some(run) => format!(
            "<a class=\"hover:opacity-70 transition-opacity\" href=\"/run/{run}\">{label}</a>",
            run = escape(run)
        ),
        None => label.to_string(),
    }
}

/// The registry, the credentials and the docker images runs are built from.
pub(crate) fn setup_page() -> std::io::Result<String> {
    let registry = registry::load()?;
    let (usage, images) = std::thread::scope(|scope| {
        let images = scope.spawn(image_rows);
        (
            usage::report(&registry),
            images.join().expect("listing the images does not panic"),
        )
    });
    let usage = usage?;

    let mut backend_rows = Vec::new();
    let mut limit_rows = Vec::new();
    let mut raw_limits = String::new();
    let mut sources: Vec<String> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    for (backend, usage) in registry.backends.iter().zip(&usage) {
        let state = if std::env::var(&backend.key).is_ok() {
            pill(PASSED_PILL, false, "set")
        } else {
            pill(BROKEN_PILL, false, "missing")
        };
        let recorded = &usage.recorded;
        backend_rows.push(vec![
            escape(&backend.name),
            backend.service.name().to_string(),
            format!(
                "<span class=\"{MONO_CLASSES}\">{}</span>",
                escape(&backend.host)
            ),
            format!(
                "<span class=\"{MONO_CLASSES}\">{}</span>",
                escape(&backend.key)
            ),
            state,
            recorded.runs.to_string(),
            recorded.analyses.to_string(),
            recorded.requests.to_string(),
            recorded.input_tokens.to_string(),
            recorded.output_tokens.to_string(),
            recorded.cache_read_tokens.to_string(),
            recorded.cache_write_tokens.to_string(),
            usage::money(recorded.gateway_cost),
        ]);

        limit_rows.extend(limit_rows_of(&backend.name, &usage.limits));
        sources.push(format!(
            "{} {}",
            escape(&backend.name),
            escape(&usage.source)
        ));
        if let Some(failure) = &usage.failure {
            failures.push(format!("{}: {}", escape(&backend.name), escape(failure)));
        }
        if !usage.limits.is_empty() {
            raw_limits.push_str(&format!(
                "<p class=\"mt-2\"><span class=\"{NOTE_CLASSES}\">{}, {}:</span> <span class=\"{MONO_CLASSES} text-xs text-neutral-300 break-all\">{}</span></p>",
                escape(&backend.name),
                escape(&usage.source),
                escape(&usage.limits)
            ));
        }
    }

    let mut model_rows = Vec::new();
    for model in &registry.models {
        for route in &model.routes {
            model_rows.push(vec![
                escape(&model.name),
                escape(&route.backend),
                format!(
                    "<span class=\"{MONO_CLASSES}\">{}</span>",
                    escape(&route.id)
                ),
                route.context_window.to_string(),
                route.max_output.to_string(),
            ]);
        }
    }

    let harness_rows = registry
        .harnesses
        .iter()
        .map(|harness| {
            vec![
                escape(&harness.name),
                harness
                    .services
                    .iter()
                    .map(|service| service.name())
                    .collect::<Vec<_>>()
                    .join(", "),
            ]
        })
        .collect();

    let mut body = format!(
        "<p class=\"{FIRST_TITLE_CLASSES}\">{}</p>",
        explained(
            "backends",
            "with the key of each and the usage recorded over every run and analysis on disk"
        )
    );
    body.push_str(&table(
        &[
            "BACKEND",
            "SERVICE",
            "HOST",
            "KEY",
            "*STATE",
            "#RUNS",
            "#ANALYSES",
            "#REQUESTS",
            "#INPUT",
            "#OUTPUT",
            "#CACHE READ",
            "#CACHE WRITE",
            "#COST",
        ],
        backend_rows,
        None,
    ));
    body.push_str(&format!(
        "<p class=\"{TITLE_CLASSES}\">{} <span class=\"{NOTE_CLASSES} font-normal\">{}</span></p>",
        explained("limits", "as each backend reports them when asked"),
        sources.join(", ")
    ));
    body.push_str(&table(
        &[
            "BACKEND",
            "WINDOW",
            "*USED",
            "LEFT",
            "STATUS",
            "*RESETS|how far the window has run towards its reset, and when it resets",
        ],
        limit_rows,
        Some(NO_LIMITS_NOTE),
    ));
    for failure in failures {
        body.push_str(&format!("<p class=\"mt-2 {NOTE_CLASSES}\">{failure}</p>"));
    }
    if !raw_limits.is_empty() {
        body.push_str(&format!(
            "<details class=\"mt-3\"><summary class=\"{SUMMARY_CLASSES}\">the raw limit headers</summary>{raw_limits}</details>"
        ));
    }
    body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">models</p>"));
    body.push_str(&table(
        &["MODEL", "BACKEND", "*ID", "#CONTEXT", "#MAX OUTPUT"],
        model_rows,
        None,
    ));
    body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">harnesses</p>"));
    body.push_str(&table(&["HARNESS", "SERVICES"], harness_rows, None));

    if let Some(rows) = images {
        body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">images</p>"));
        body.push_str(&table(&["IMAGE", "TAG", "SIZE", "CREATED"], rows, None));
    }

    Ok(page("setup", &body))
}

/// The images of ava as table rows, or nothing when docker does not answer.
fn image_rows() -> Option<Vec<Vec<String>>> {
    let listing =
        process::run_and_assume_success("docker", &["image", "ls", "--format", IMAGE_FORMAT])
            .ok()?;

    Some(
        listing
            .lines()
            .filter(|line| line.starts_with(IMAGE_PREFIX))
            .map(|line| line.split('\t').map(escape).collect())
            .collect(),
    )
}

/// A page carrying one failure, for the errors of the reading views.
pub(crate) fn error_page(message: &str) -> String {
    let body = format!(
        "<p class=\"max-w-prose rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-red-300\">{}</p><p class=\"mt-4\"><a class=\"{LINK_CLASSES}\" href=\"/\">back to the runs</a></p>",
        escape(message)
    );

    page("error", &body)
}

/// The known game folders, sorted.
pub(crate) fn games() -> std::io::Result<Vec<String>> {
    let mut games: Vec<String> = std::fs::read_dir(GAMES_DIRECTORY)
        .map_err(|error| at_path(GAMES_DIRECTORY, error))?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| docker::task_directory(name, 0).is_dir())
        .collect();
    games.sort();

    Ok(games)
}

/// The contents of one of the known files of a run.
pub(crate) fn run_file(name: &str, file: &str) -> Option<Vec<u8>> {
    if !RUN_FILES.contains(&file) {
        return None;
    }

    std::fs::read(run_directory(name).ok()?.join(file)).ok()
}

/// The entry a run kept from the attempt at `seconds`, which is the entry file
/// of its game and nothing else.
pub(crate) fn run_entry(name: &str, seconds: &str, file: &str) -> Option<Vec<u8>> {
    let seconds: u64 = seconds.parse().ok()?;
    let directory = run_directory(name).ok()?;
    let run = runs::read(&directory).ok()?;
    let game = ava_game::find(&run.game)?;
    if file != runs::kept_file(game, &run) {
        return None;
    }

    std::fs::read(
        directory
            .join(docker::ENTRIES_DIRECTORY)
            .join(seconds.to_string())
            .join(file),
    )
    .ok()
}

/// The directory of the named run, refusing any name that is not a plain
/// directory entry under the run directory.
pub(crate) fn run_directory(name: &str) -> std::io::Result<std::path::PathBuf> {
    if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
        return Err(std::io::Error::other(format!("{name}: not a run name")));
    }

    let directory = std::path::Path::new(docker::RUN_DIRECTORY).join(name);
    if !directory.is_dir() {
        return Err(std::io::Error::other(format!("{name}: no such run")));
    }

    Ok(directory)
}

/// The files of a tournament the file route hands out: the record and the
/// console of every round.
fn tournament_files(name: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(tournament::directory(name)) else {
        return Vec::new();
    };

    let mut files: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|file| tournament_file_allowed(file))
        .collect();
    files.sort();
    files
}

/// Whether `file` is one the tournament file route hands out.
fn tournament_file_allowed(file: &str) -> bool {
    file == tournament::RECORD_FILE
        || file
            .strip_prefix(ROUND_LOG_PREFIX)
            .and_then(|rest| rest.strip_suffix(ROUND_LOG_SUFFIX))
            .is_some_and(|number| {
                !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
            })
}

/// The contents of one of the files of a tournament.
pub(crate) fn tournament_file(name: &str, file: &str) -> Option<Vec<u8>> {
    tournament::load(name).ok()?;
    if !tournament_file_allowed(file) {
        return None;
    }

    std::fs::read(tournament::directory(name).join(file)).ok()
}

/// Whether an analyst is up for the named run.
pub(crate) fn analyzing(name: &str) -> bool {
    live_runs()
        .iter()
        .any(|container| container == &docker::analyst_container(name))
}

/// What the watcher last saw of one live run, read out of the output of its
/// containers once per look, so no page parses that output itself.
#[derive(Clone)]
struct LiveRun {
    run: String,
    /// The pushes its scoring container graded so far.
    attempts: Vec<ava_wire::Attempt>,
    /// The requests its proxy served so far, aggregated, or nothing when that
    /// output does not aggregate.
    metrics: Option<ava_wire::Metrics>,
}

/// What the watcher last saw of docker: the running containers and the state
/// of every live run.
struct Snapshot {
    containers: Vec<String>,
    live: Vec<LiveRun>,
}

static SNAPSHOT: std::sync::Mutex<Snapshot> = std::sync::Mutex::new(Snapshot {
    containers: Vec::new(),
    live: Vec::new(),
});

/// How long the watcher rests between two looks at docker.
const CONTAINER_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// Keep the snapshot fresh from a thread, so no page waits on docker.
pub(crate) fn watch_containers() {
    std::thread::spawn(|| {
        loop {
            refresh_snapshot();
            std::thread::sleep(CONTAINER_POLL_INTERVAL);
        }
    });
}

fn refresh_snapshot() {
    let containers: Vec<String> =
        process::run_and_assume_success("docker", &["ps", "--format", "{{.Names}}"])
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect();
    let live = containers
        .iter()
        .filter_map(|container| container.strip_prefix(docker::SCORER_CONTAINER_PREFIX))
        .map(|run| LiveRun {
            run: run.to_string(),
            attempts: attempt_lines(&container_logs(&docker::scorer_container(run))),
            metrics: aggregated(run, &container_logs(&docker::proxy_container(run))),
        })
        .collect();

    *SNAPSHOT.lock().expect("the snapshot is never poisoned") = Snapshot { containers, live };
}

/// What the watcher last saw of the live run `name`, if it saw it.
fn live_run(name: &str) -> Option<LiveRun> {
    SNAPSHOT
        .lock()
        .expect("the snapshot is never poisoned")
        .live
        .iter()
        .find(|live| live.run == name)
        .cloned()
}

/// The metrics of the live run `name` over `logged`, what its proxy printed up
/// to this look, or nothing when that does not aggregate. A request is logged
/// when it completes, so the one in flight is not counted yet.
fn aggregated(name: &str, logged: &str) -> Option<ava_wire::Metrics> {
    // A record still being written is not a record yet.
    let whole: String = logged
        .lines()
        .filter(|line| line.starts_with('{') && line.ends_with('}'))
        .map(|line| format!("{line}\n"))
        .collect();

    ava_scorer::score::aggregate(&whole)
        .inspect_err(|error| log::warn!("{name}: the proxy log so far does not aggregate: {error}"))
        .ok()
}

fn container_logs(container: &str) -> String {
    std::process::Command::new("docker")
        .args(["logs", container])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

/// The names of the running containers.
fn live_runs() -> Vec<String> {
    SNAPSHOT
        .lock()
        .expect("the snapshot is never poisoned")
        .containers
        .clone()
}

/// Every run on disk, newest first, marked live while its scoring container is up,
/// which outlives the agent container restarting between turns.
fn collect_runs() -> std::io::Result<Vec<RunEntry>> {
    let running = live_runs();
    let placements = tournament::placements()?;

    let mut runs: Vec<RunEntry> = runs::all()?
        .into_iter()
        .map(|(directory, run)| RunEntry::new(&directory, run, &running, &placements))
        .collect();

    runs.sort_by_key(|run| std::cmp::Reverse(run.run.started_seconds));

    Ok(runs)
}

/// The graded pushes of a finished run: the record, or the collected log for a
/// run that broke before its record was completed.
fn attempts_of(directory: &std::path::Path, run: &ava_wire::Run) -> Vec<ava_wire::Attempt> {
    if !run.attempts.is_empty() {
        return run.attempts.clone();
    }

    attempt_lines(&std::fs::read_to_string(directory.join(docker::SCORE_LOG)).unwrap_or_default())
}

/// The attempts among `lines`, one JSON record each, as the scoring container
/// prints them and the attempts log keeps them.
fn attempt_lines(lines: &str) -> Vec<ava_wire::Attempt> {
    lines
        .lines()
        .filter(|line| line.starts_with('{'))
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// The graded pushes as table rows.
fn attempt_rows(attempts: &[ava_wire::Attempt]) -> Vec<Vec<String>> {
    attempts
        .iter()
        .map(|attempt| {
            vec![
                attempt.seconds.to_string(),
                if attempt.verdict.passed {
                    pill(PASSED_PILL, false, "passed")
                } else {
                    pill(FAILED_PILL, false, "failed")
                },
                escape(attempt.verdict.reason.as_deref().unwrap_or_default()),
            ]
        })
        .collect()
}

/// A flat JSON object as a two column table in a card.
fn object_table(value: &serde_json::Value) -> String {
    let Some(object) = value.as_object() else {
        return String::new();
    };

    let rows: Vec<(String, String)> = object
        .iter()
        .map(|(key, value)| (key.clone(), plain(value)))
        .collect();
    pairs_table(&rows, MONO_VALUE_CLASSES)
}

/// A two column table of labels and values without a header, the values in
/// `value_classes`.
fn pairs_table(rows: &[(String, String)], value_classes: &str) -> String {
    let mut html = format!(
        "<div class=\"{CARD_CLASSES} overflow-hidden\"><table class=\"{TABLE_CLASSES}\"><tbody>"
    );
    for (index, (label, value)) in rows.iter().enumerate() {
        let border = if index == 0 { "border-t-0" } else { "" };
        html.push_str(&format!(
            "<tr class=\"{ROW_CLASSES}\"><td class=\"{PACKED_COLUMN_CLASSES} {CELL_CLASSES} {border} align-top text-neutral-400\">{}</td><td class=\"{SLACK_COLUMN_CLASSES} {CELL_CLASSES} {border} {value_classes}\">{}</td></tr>",
            escape(label),
            escape(value)
        ));
    }
    html.push_str("</tbody></table></div>");
    html
}

/// A JSON value as one displayable line, fractions cut short.
fn plain(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(items) => items.iter().map(plain).collect::<Vec<_>>().join(" "),
        serde_json::Value::Number(number) => match number.as_f64() {
            Some(float) if float.fract() != 0.0 => format!("{float:.3}"),
            _ => number.to_string(),
        },
        other => other.to_string(),
    }
}

/// The error of a failed path operation, with the path it was given.
///
/// The bare error of a syscall names the reason and never the path, which
/// leaves a reader of the message guessing which file was meant.
fn at_path(path: &str, error: std::io::Error) -> std::io::Error {
    std::io::Error::new(error.kind(), format!("{path}: {error}"))
}

fn read_json(path: &std::path::Path) -> Option<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn number(value: &serde_json::Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

/// The last `CONSOLE_TAIL_BYTES` of the file at `path`, read without the rest.
fn console_tail(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let start = file
        .metadata()?
        .len()
        .saturating_sub(CONSOLE_TAIL_BYTES as u64);
    std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(start))?;

    let mut tail = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut tail)?;
    Ok(tail)
}

/// The limit rows of one backend out of its `name=value` pairs.
fn limit_rows_of(backend: &str, limits: &str) -> Vec<Vec<String>> {
    let backend_cell = format!("<span class=\"{MONO_CLASSES}\">{}</span>", escape(backend));

    usage::lines(limits)
        .into_iter()
        .map(|line| {
            let used = match line.used {
                Some((used, ceiling)) => meter(
                    used,
                    ceiling,
                    USAGE_FILL,
                    &escape(&line.used_label),
                    USAGE_LABEL_WIDTH,
                ),
                None => String::new(),
            };
            vec![
                backend_cell.clone(),
                escape(&line.window),
                used,
                escape(&line.left),
                status_pill(&line.status),
                match line.wait {
                    Some((left, window)) => meter(
                        window.saturating_sub(left),
                        window,
                        WAIT_FILL,
                        &escape(&line.resets),
                        RESET_LABEL_WIDTH,
                    ),
                    None => escape(&line.resets),
                },
            ]
        })
        .collect()
}

/// A limit status as a pill, with what qualifies it muted behind: allowed is
/// fine, a warning is amber, a rejection is red, anything else is neutral.
fn status_pill(status: &str) -> String {
    let (status, note) = status.split_once(' ').unwrap_or((status, ""));
    if status.is_empty() {
        return String::new();
    }

    let tint = match status {
        "allowed" => PASSED_PILL,
        "allowed_warning" => STARTING_PILL,
        "rejected" => BROKEN_PILL,
        _ => NEUTRAL_PILL,
    };
    let note = if note.is_empty() {
        String::new()
    } else {
        format!(" <span class=\"{MUTED_CLASSES}\">{}</span>", escape(note))
    };

    format!("{}{note}", pill(tint, false, &escape(status)))
}

/// The harness with its thinking level, the way an agent is referred to.
fn agent_label(harness: &str, thinking: &str) -> String {
    if thinking.is_empty() {
        return escape(harness);
    }

    format!(
        "<span class=\"whitespace-nowrap\">{} <span class=\"{MUTED_CLASSES}\">{}</span></span>",
        escape(harness),
        escape(thinking)
    )
}

/// A version muted beside the thing it versions, or nothing when none was recorded.
fn version_label(version: &str) -> String {
    if version.is_empty() {
        return String::new();
    }

    format!(
        "<span class=\"{MUTED_CLASSES} {MONO_CLASSES} text-xs\">{}</span>",
        escape(version)
    )
}

/// The seat a run plays in its round, and its turn for a game with several.
fn placement_role(placement: &tournament::Placement, turns: usize) -> String {
    let turn = if turns > 1 {
        format!(", turn {}", placement.turn + 1)
    } else {
        String::new()
    };

    format!(
        "seat {} in round {}{turn}",
        placement.seat + 1,
        placement.round + 1
    )
}

/// Where a run sits in a tournament, linking the tournament.
fn placement_label(placement: &tournament::Placement, turns: usize) -> String {
    format!(
        "{} of {}",
        placement_role(placement, turns),
        tournament_link(&placement.tournament)
    )
}

/// The name of a tournament as a link into its page.
fn tournament_link(name: &str) -> String {
    format!(
        "<a class=\"{LINK_CLASSES}\" href=\"/tournament/{name}\">{name}</a>",
        name = escape(name)
    )
}

/// The game of a run, with the task of its turn when it is not the first.
fn game_label(run: &ava_wire::Run) -> String {
    if run.turn > 0 {
        return format!(
            "{} <span class=\"{MUTED_CLASSES}\">{}</span>",
            escape(&run.game),
            escape(
                ava_game::find(&run.game)
                    .map(|game| runs::turn_task(game, run.turn))
                    .unwrap_or_default()
            )
        );
    }

    escape(&run.game)
}

/// The name of a run as a link into its page.
fn run_link(name: &str) -> String {
    format!(
        "<a class=\"{LINK_CLASSES}\" href=\"/run/{name}\">{name}</a>",
        name = escape(name)
    )
}

/// A tile of the run page: a label over a value, with a muted detail beneath
/// it when there is one.
fn tile(label: &str, value: &str, detail: &str, value_classes: &str) -> String {
    let detail = if detail.is_empty() {
        String::new()
    } else {
        format!("<p class=\"{TILE_DETAIL_CLASSES}\">{detail}</p>")
    };

    format!(
        "<div class=\"{TILE_CLASSES}\"><p class=\"{TILE_LABEL_CLASSES}\">{label}</p><div class=\"{value_classes}\">{value}</div>{detail}</div>"
    )
}

/// A row of tiles.
fn tiles(tiles: &[String]) -> String {
    format!(
        "<div class=\"{TILE_GRID_CLASSES}\">{}</div>",
        tiles.concat()
    )
}

/// What a tile shows instead of a value it does not have, and why.
fn placeholder(reason: &str) -> String {
    format!("<span class=\"{PLACEHOLDER_CLASSES}\">{reason}</span>")
}

/// The non-empty `parts`, escaped and joined by a dot.
fn joined(parts: &[&str]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .map(|part| escape(part))
        .collect::<Vec<_>>()
        .join(" \u{00b7} ")
}

/// A state pill, with a pulsing dot for a state still changing.
fn pill(tint: &str, pulsing: bool, label: &str) -> String {
    let dot = if pulsing {
        "<span class=\"h-1.5 w-1.5 translate-y-px rounded-full bg-current animate-pulse\"></span>"
    } else {
        ""
    };

    format!("<span class=\"{PILL_CLASSES} {tint}\">{dot}{label}</span>")
}

/// A label with its explanation behind a tooltip, or bare without one.
fn explained(label: &str, tooltip: &str) -> String {
    if tooltip.is_empty() {
        return label.to_string();
    }

    format!(
        "<span class=\"{TOOLTIP_CLASSES}\" title=\"{}\">{label}</span>",
        escape(tooltip)
    )
}

/// The fill of a time meter.
fn time_fill(spent: u64, limit: u64) -> &'static str {
    if spent >= limit {
        TIME_SPENT_FILL
    } else {
        TIME_LEFT_FILL
    }
}

/// A points value behind its meter on the shared 0 to 10000 scale.
fn points_meter(points: u64) -> String {
    meter(
        points,
        ava_game::MAXIMUM_POINTS,
        POINTS_FILL,
        &points.to_string(),
        POINTS_LABEL_WIDTH,
    )
}

/// `value` out of `ceiling` as a meter filling its cell, with `label` in a
/// column of `label_width` beside it.
fn meter(value: u64, ceiling: u64, fill: &str, label: &str, label_width: &str) -> String {
    let percent = (value.min(ceiling) * 100).checked_div(ceiling).unwrap_or(0);

    format!(
        "<span class=\"flex items-center gap-2 whitespace-nowrap\">\
         <span class=\"{METER_TRACK_CLASSES}\"><span class=\"block h-full rounded-full {fill}\" style=\"width:{percent}%\"></span></span>\
         <span class=\"{MONO_CLASSES} tabular-nums text-neutral-200 shrink-0 {label_width}\">{label}</span></span>"
    )
}

/// A table in a card whose `#` marked headers hold right-aligned numbers and
/// whose `*` marked headers share the slack evenly. Without rows it shows
/// `empty`, or nothing when there is no note to show.
fn table(headers: &[&str], rows: Vec<Vec<String>>, empty: Option<&str>) -> String {
    if rows.is_empty() && empty.is_none() {
        return String::new();
    }

    let numeric: Vec<bool> = headers
        .iter()
        .map(|header| header.starts_with(NUMERIC_MARKER))
        .collect();
    let mut slack: Vec<usize> = headers
        .iter()
        .enumerate()
        .filter(|(_, header)| header.starts_with(SLACK_MARKER))
        .map(|(index, _)| index)
        .collect();
    if slack.is_empty() {
        slack.push(headers.len().saturating_sub(1));
    }
    let column = |index: usize| {
        if slack.contains(&index) {
            SLACK_COLUMN_CLASSES
        } else {
            PACKED_COLUMN_CLASSES
        }
    };

    let mut html = format!(
        "<div class=\"{CARD_CLASSES} overflow-x-auto\"><table class=\"{TABLE_CLASSES}\"><thead><tr>"
    );
    for (index, (header, numeric)) in headers.iter().zip(&numeric).enumerate() {
        let align = if *numeric { "text-right" } else { "text-left" };
        let classes = column(index);
        let (title, tooltip) = header.split_once(TOOLTIP_SEPARATOR).unwrap_or((header, ""));
        html.push_str(&format!(
            "<th class=\"{classes} {HEADER_CLASSES} {align}\">{}</th>",
            explained(
                title.trim_start_matches([NUMERIC_MARKER, SLACK_MARKER]),
                tooltip
            )
        ));
    }
    html.push_str("</tr></thead><tbody>");

    if rows.is_empty() {
        html.push_str(&format!(
            "<tr><td colspan=\"{}\" class=\"{EMPTY_ROW_CLASSES}\">{}</td></tr>",
            headers.len(),
            empty.unwrap_or_default()
        ));
    }

    for row in rows {
        html.push_str(&format!("<tr class=\"{ROW_CLASSES}\">"));
        for (index, (cell, numeric)) in row.iter().zip(&numeric).enumerate() {
            let align = if *numeric { NUMERIC_CLASSES } else { "" };
            let classes = column(index);
            html.push_str(&format!(
                "<td class=\"{classes} {CELL_CLASSES} {align}\">{cell}</td>"
            ));
        }
        html.push_str("</tr>");
    }

    html.push_str("</tbody></table></div>");
    html
}

/// The layout around one rendered `body`, headed by `heading`.
fn page(heading: &str, body: &str) -> String {
    LAYOUT_TEMPLATE
        .replace(HEADING_PLACEHOLDER, &escape(heading))
        .replace(BODY_PLACEHOLDER, body)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Drop the ANSI escape sequences agent consoles are full of.
fn strip_ansi(text: &str) -> String {
    let mut stripped = String::with_capacity(text.len());
    let mut characters = text.chars();

    while let Some(character) = characters.next() {
        if character != '\u{1b}' {
            stripped.push(character);
            continue;
        }

        match characters.next() {
            // A control sequence runs until its final letter.
            Some('[') => {
                for next in characters.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // An operating system command runs until its bell.
            Some(']') => {
                for next in characters.by_ref() {
                    if next == '\u{7}' {
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    stripped
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_cover_tells_text_from_bytes() {
        assert!(super::is_text(b"; a warrior\nmov eax, 1\n"));
        assert!(!super::is_text(b"\x7fELF\x02\x01"));
        assert!(!super::is_text(b""));
        assert_eq!(
            super::task_title("# Sanity check\n\nSubmit"),
            "Sanity check"
        );
        assert_eq!(super::task_title("no heading"), "");
    }

    #[test]
    fn no_cover_comes_from_outside_the_games_directory() {
        assert!(super::cover_path("../Cargo.toml").is_none());
        assert!(super::game_cover("no-such-game").is_none());
    }
}
