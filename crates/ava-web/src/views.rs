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

/// What the analysis settings offer preselected beside the analyst of the registry.
const DEFAULT_ANALYST_THINKING: &str = "medium";

/// The avatar of an agent: a mirrored grid of cells lit by the bits of a hash
/// of harness and model, in a hue the hash picks.
const AVATAR_SIDE: u64 = 5;
const AVATAR_COLUMNS: u64 = 3;
const AVATAR_HUES: u64 = 360;
const AVATAR_CLASSES: &str = "h-6 w-6 rounded";

/// The avatar on the agent tile of a run, the height of the line beside it.
const AGENT_TILE_AVATAR_CLASSES: &str = "h-5 w-5 shrink-0 rounded";
const AVATAR_GROUND_CLASSES: &str = "fill-neutral-800";
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

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

/// A `^` marked header holds a centered column. The markers combine: `*#`
/// is a right-aligned column taking slack, which pushes what follows it to
/// the right edge.
const CENTER_MARKER: char = '^';
const MARKERS: [char; 3] = [NUMERIC_MARKER, SLACK_MARKER, CENTER_MARKER];

/// Whatever follows a `|` in a header is the tooltip explaining that column
/// rather than part of its title.
const TOOLTIP_SEPARATOR: char = '|';

/// The unified runs table, holding pending, live and finished runs alike.
const RUN_HEADERS: [&str; 10] = [
    "agent|the agent that played the run, with the harness and the thinking level it was asked \
     for",
    "run|the run under runs/, named after the agent that played it, and how long ago it started",
    "state|live or the last call while the run goes, whether a push passed the verifier once it is over",
    "analysis|what became of the analysis the run is due: pending until the analyst starts, then \
     analyzing, analyzed or failed, a dash for a run started without one",
    "game|the game that was played",
    "tournament|the tournament the run plays a seat in, or a dash for a run of its own",
    "model|the model under test",
    "*time|seconds spent of the time budget, red once the whole budget is gone",
    "*points|the entry of record ranked on the 0 to 10000 scale every game ranks in, once the run \
     is over",
    "",
];
const NO_RUNS_NOTE: &str = "no runs yet, start one above";
/// What the tournament column shows for a run of its own, and the run and
/// state columns for a start that has not reached the disk.
const NO_TOURNAMENT: &str = "-";
const NO_RUN_YET: &str = "-";
const PENDING_STATE: &str = "pending";
const NO_LIMITS_NOTE: &str = "no backend reported its limits";
const NO_TOURNAMENTS_NOTE: &str = "no tournaments yet, open one above";
const NO_SEATS_NOTE: &str = "no seats yet, seat an agent below";

/// The names the script keeps the sort of a table under.
const STANDINGS_TABLE: &str = "standings";
const AGENTS_TABLE: &str = "agents";
const RIVALS_TABLE: &str = "rivals";

/// The columns of the standings after the cross table of the seats.
const STANDINGS_HEADERS: [&str; 5] = [
    "*#fights|the fights against another agent as won-drawn-lost, a fight with more rounds won \
     than lost is won",
    "#score|the share of the rounds of those fights won, half for a draw, what the ratings are fed",
    "#elo|updated in match order, anchored at 1000",
    "#bradley-terry|fitted over the whole history, anchored at 1000",
    "",
];

/// The column of [`STANDINGS_HEADERS`] the standings sort by.
const STANDINGS_SCORE_COLUMN: usize = 1;

/// The score a seat without one sorts at.
const UNRATED_SCORE: f64 = f64::NEG_INFINITY;
const NO_AGENTS_NOTE: &str = "no agents";
const NO_AGENTS_ROW_NOTE: &str = "no agents";

/// The columns of the rivals table and the one it sorts by.
const RIVALS_HEADERS: [&str; 5] = [
    "",
    "agent",
    "*rounds|the rounds against that agent as won-drawn-lost, over the finished rounds of every \
     tournament",
    "#fought|the pairings the two played",
    "#score|the share of those rounds won, half for a draw",
];
const RIVALS_FOUGHT_COLUMN: usize = 3;
const NO_RIVALS_NOTE: &str = "no tournament rounds against another agent yet";
const NO_AGENT_RUNS_NOTE: &str = "no runs yet";

/// The last runs of an agent, oldest first, as one square each.
const FORM_RUNS: usize = 40;
const FORM_CLASSES: &str = "flex flex-wrap gap-1";
const FORM_MARK_CLASSES: &str = "block h-5 w-5 rounded-sm transition-colors";
const FORM_PASSED: &str = "bg-emerald-500/70 hover:bg-emerald-400";
const FORM_FAILED: &str = "bg-orange-500/60 hover:bg-orange-400";
const FORM_BROKEN: &str = "bg-red-500/50 hover:bg-red-400";
const FORM_LIVE: &str = "bg-neutral-600 animate-pulse";

/// The avatar over the name on the page of an agent.
const PROFILE_AVATAR_CLASSES: &str = "h-10 w-10 shrink-0 rounded-md";
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
/// The header row is a darker band, its titles lowercase in a heavier weight.
const HEADER_ROW_CLASSES: &str = "bg-neutral-950/50";
const HEADER_CLASSES: &str = "text-xs font-semibold text-neutral-300 py-2.5";

/// The header of a sortable table sorts it, the sorted one carrying an arrow.
const SORTABLE_HEADER_CLASSES: &str = "cursor-pointer select-none hover:text-neutral-100 \
     transition-colors";
const SORT_ARROW_CLASSES: &str = "ml-1 text-neutral-500";
const DESCENDING_ARROW: &str = "\u{2193}";
const ASCENDING_ARROW: &str = "\u{2191}";
const DESCENDING_ORDER: &str = "desc";
const ASCENDING_ORDER: &str = "asc";
/// How the script reads a column: the first number in a cell, or its text.
const NUMERIC_SORT: &str = "numeric";
const TEXT_SORT: &str = "text";

/// A title with a tooltip behind it.
const TOOLTIP_CLASSES: &str = "cursor-help underline decoration-dotted decoration-neutral-700 \
     underline-offset-4 hover:text-neutral-300 hover:decoration-neutral-500 transition-colors";
const CELL_CLASSES: &str = "py-2.5 border-t border-neutral-800 align-middle";
const ROW_CLASSES: &str = "hover:bg-neutral-800/40 transition-colors";

/// A cell that is a link, filling the cell so the whole of it leads there.
const CELL_LINK_CLASSES: &str = "block hover:text-neutral-100 transition-colors";
const NUMERIC_CLASSES: &str = "text-right font-mono tabular-nums";
const CENTERED_CLASSES: &str = "text-center";
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

/// The chat box: prose, so it wraps, and a screenful, so it scrolls.
const CHAT_CLASSES: &str = "rounded-lg border border-neutral-800 bg-neutral-950 p-4 text-xs \
     font-mono leading-relaxed text-neutral-300 whitespace-pre-wrap break-words \
     overflow-y-auto min-h-24 max-h-56";

/// What the line above the chat box says while the run is live, in the margins
/// of a title without its weight.
const THINKING_CLASSES: &str = "text-sm italic text-neutral-400 mt-8 mb-3";
const THINKING_NOTE: &str = "is thinking..";

/// The caret trailing the generated text.
const CARET_CLASSES: &str = "text-indigo-400 animate-pulse";
const CARET: &str = "\u{258c}";

/// A ring turning while a run generates.
const SPINNER_CLASSES: &str = "inline-block h-3 w-3 align-[-1px] rounded-full border-2 \
     border-neutral-700 border-t-indigo-400 animate-spin";
/// A section title that folds its section.
const COLLAPSIBLE_TITLE_CLASSES: &str = "cursor-pointer list-none [&::-webkit-details-marker]:hidden \
     text-sm font-semibold text-neutral-100 mt-8 mb-3";

/// The renderer caps its prose at a reading width. Inside a box that is the
/// width, the prose fills the box and wraps at its edge.
const FULL_WIDTH_PROSE: &str = "[&_*]:max-w-none";
const SUMMARY_CLASSES: &str = "cursor-pointer list-none [&::-webkit-details-marker]:hidden \
     text-neutral-400 hover:text-neutral-200 transition-colors";

/// One height for every form control, so a row of them shares a baseline. It
/// fits the two lines of the agent picker.
const CONTROL_HEIGHT: &str = "h-11";
const BUTTON_CLASSES: &str =
    "rounded-md bg-indigo-500 hover:bg-indigo-400 px-4 font-medium text-white transition-colors";
const STOP_CLASSES: &str = "rounded-md border border-red-500/40 text-red-400 hover:bg-red-500/10 \
     px-2.5 py-1 text-xs font-medium transition-colors";
const FIELD_CLASSES: &str = "w-full rounded-md border border-neutral-700 bg-neutral-950 px-3 \
     text-neutral-100 focus:outline-none focus:border-indigo-500 focus:ring-2 \
     focus:ring-indigo-500/30 transition";

/// The agent picker: a field showing the choice, folding out a list with one
/// row per agent. A row is the avatar beside two lines, the name over the
/// harness, model and backend in small gray, so the choice reads whole.
const PICKER_CLASSES: &str = "relative grow basis-64";
const PICKER_FACE_CLASSES: &str = "flex items-center gap-2 cursor-pointer list-none \
     [&::-webkit-details-marker]:hidden";
const PICKER_LIST_CLASSES: &str = "absolute z-10 mt-1 min-w-full w-max max-w-2xl max-h-80 \
     overflow-y-auto rounded-md border border-neutral-700 bg-neutral-950 py-1 shadow-lg";
const PICKER_ROW_CLASSES: &str = "flex items-center gap-2 px-3 py-2 cursor-pointer \
     hover:bg-neutral-800 has-[:checked]:bg-neutral-800/60";
const PICKER_LINES_CLASSES: &str = "flex flex-col min-w-0 leading-tight";

/// The field an open picker filters its rows by, above them as they scroll.
const PICKER_FILTER_CLASSES: &str = "sticky top-0 z-10 w-full bg-neutral-950 border-b \
     border-neutral-800 px-3 py-2 text-neutral-100 placeholder:text-neutral-600 \
     focus:outline-none";
const PICKER_FILTER_NOTE: &str = "type to filter";
const PICKER_CONFIGURATION_CLASSES: &str = "text-xs text-neutral-500 whitespace-nowrap";
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
const TILE_LABEL_CLASSES: &str = "text-xs font-medium text-neutral-500";
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
const FACT_VALUE_CLASSES: &str = "font-mono tabular-nums text-neutral-100";
const FACT_WORDS_CLASSES: &str = "text-neutral-100";
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
/// The dot sits before the state word, whose width is estimated per character.
const GRAPH_CHARACTER_WIDTH: f64 = 6.6;
const GRAPH_DOT_GAP: f64 = 9.0;
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
/// What is not played yet is drawn faded and its edges dashed.
const GRAPH_PLANNED_CLASSES: &str = "opacity-40";
const GRAPH_PLANNED_DASH: &str = "4 4";
const GRAPH_PLANNED_STATE: &str = "pending";
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
const HOLDER_CLASSES: &str = "block h-4 mt-1 pl-[4.25rem] text-xs text-neutral-400 truncate";
const FACT_LABEL_CLASSES: &str = "w-14 shrink-0 text-xs text-neutral-400";

/// The tournament card beside the game card: the lobby as the numbered seats
/// on a ring with a line for every pairing.
const ABOUT_GRID_CLASSES: &str = "mt-4 grid grid-cols-2 gap-4 items-stretch";
const RING_SIDE: f64 = 80.0;
const RING_RADIUS: f64 = 30.0;
const RING_SEAT_RADIUS: f64 = 8.0;
const RING_DOT_RADIUS: f64 = 3.5;
const RING_STROKE_WIDTH: f64 = 1.0;
const RING_NUMBER_SIZE: f64 = 9.0;
/// A lobby of more seats than this shows them as dots, too close for numbers.
const RING_NUMBERED_SEATS: usize = 8;
const RING_EDGE_CLASSES: &str = "stroke-neutral-600";
const RING_SEAT_CLASSES: &str = "fill-neutral-800 stroke-neutral-500";
const RING_NUMBER_CLASSES: &str = "fill-neutral-200 font-mono";
const RING_DOT_CLASSES: &str = "fill-neutral-300";
const FACT_TEXT_CLASSES: &str = "flex items-center gap-2 whitespace-nowrap";
const NO_ANALYST: &str = "none";
/// The header of every page is its section, the run or the tournament being
/// the title of the body. The settings card holds what was fixed when the
/// tournament opened.
const RUNS_HEADING: &str = "runs";
const TOURNAMENTS_HEADING: &str = "tournaments";
const AGENTS_HEADING: &str = "agents";
const SETTINGS_TITLE: &str = "settings";

/// A tile with nothing to show says why, quietly.
const PLACEHOLDER_CLASSES: &str = "text-sm font-normal text-neutral-500";
const AFTER_THE_RUN: &str = "after the run";
const NOT_RECORDED: &str = "not recorded";
const NO_ENTRY: &str = "no entry";

/// A passing run from before entries were kept left no entry file.
const NOT_KEPT: &str = "not kept";
/// What the image fact of a game without a layer of its own says.
const BASE_IMAGE: &str = "base";
const UNRANKED: &str = "unranked";
const UNFINISHED: &str = "unfinished";
const NOT_ANALYZED: &str = "-";

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
    fn agent<'a>(&'a self, prefix: &str, defaults: [&'a str; 2]) -> [&'a str; 2] {
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
    /// The name the agent was picked under.
    pub name: String,
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
                run.analyst.is_some() && (live || run.finished_seconds.is_some()),
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

    /// Whether `alias` played the run: the name it was started under, else the
    /// pairing, for a record from before the names.
    fn by(&self, alias: &registry::Alias) -> bool {
        match &self.run.agent_name {
            Some(name) => *name == alias.name,
            None => self.run.agent() == alias.agent(),
        }
    }

    /// The name the agent of the run goes by.
    fn agent_title(&self, registry: &registry::Registry) -> String {
        self.run
            .agent_name
            .clone()
            .unwrap_or_else(|| agent_name(registry, &self.run.agent()))
    }

    /// The points of the entry of record, nothing for a game ranking nothing.
    fn points(&self) -> Option<u64> {
        self.record.as_ref().and_then(|entry| entry.points)
    }

    /// The agent cell of the runs table, leading to the run it played.
    fn agent_cell(&self, registry: &registry::Registry) -> String {
        format!(
            "<a class=\"{CELL_LINK_CLASSES}\" href=\"/run/{}\">{}</a>",
            escape(&self.name),
            agent_stack(
                self.agent_title(registry),
                &self.run.agent(),
                self.run.thinking.as_deref().unwrap_or_default()
            )
        )
    }

    /// Whether an analyst is up for the run.
    fn analyzing(&self) -> bool {
        matches!(self.analysis, Analysis::Analyzing)
    }

    /// The state of the analysis as a pill, nothing when none is due.
    fn analysis_pill(&self) -> String {
        match self.analysis {
            Analysis::Pending => pill(NEUTRAL_PILL, false, "pending"),
            Analysis::Analyzing => pill(STARTING_PILL, true, "analyzing"),
            Analysis::Done => pill(ANALYZED_PILL, false, "analyzed"),
            Analysis::Failed => pill(BROKEN_PILL, false, "failed"),
            Analysis::None => String::new(),
        }
    }

    /// The analysis cell of the runs table: the pill, or a dash without an analyst.
    fn analysis_cell(&self) -> String {
        match self.analysis {
            Analysis::None => placeholder(NOT_ANALYZED),
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

    /// The run cell of the runs table: what the run is called after its
    /// agent, linked, with the age beneath it.
    fn run_cell(&self) -> String {
        let id = self
            .name
            .strip_prefix(&format!("{}-", self.run.harness))
            .unwrap_or(&self.name);

        format!(
            "<a class=\"{LINK_CLASSES} block text-xs\" href=\"/run/{}\">{}\
             <div class=\"{MUTED_CLASSES} mt-0.5\">{} ago</div></a>",
            escape(&self.name),
            escape(id),
            usage::age(self.run.started_seconds)
        )
    }

    /// The tournament the run plays a seat in, linked, or a dash for a run of
    /// its own.
    fn tournament_cell(&self) -> String {
        match &self.placement {
            Some(placement) => tournament_link(&placement.tournament),
            None => format!("<span class=\"{MUTED_CLASSES}\">{NO_TOURNAMENT}</span>"),
        }
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
    fn row(&self, registry: &registry::Registry) -> Vec<String> {
        vec![
            self.agent_cell(registry),
            self.run_cell(),
            self.state(),
            self.analysis_cell(),
            game_label(&self.run),
            self.tournament_cell(),
            escape(&self.run.model),
            self.time_cell(),
            self.points_cell(),
            self.stop_form(),
        ]
    }
}

/// What became of the analysis of a run.
#[derive(Clone, Copy)]
enum Analysis {
    /// The run was started without an analyst.
    None,
    /// The analyst the run records has not started.
    Pending,
    /// An analyst is up on the run.
    Analyzing,
    /// The record holds the report.
    Done,
    /// The record holds the reason the analysis failed.
    Failed,
}

/// What became of the analysis of the run in `directory`, given whether an
/// analyst is up on it and whether one is `due`.
fn analysis_of(directory: &std::path::Path, analyzing: bool, due: bool) -> Analysis {
    if analyzing {
        return Analysis::Analyzing;
    }

    match runs::analysis(directory) {
        Ok(None) if due => Analysis::Pending,
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

    let registry = registry::load()?;
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
                agent_stack(
                    start.name.clone(),
                    &ava_wire::Agent {
                        harness: start.agent.clone(),
                        model: start.model.clone(),
                    },
                    &start.thinking,
                ),
                format!(
                    "<span class=\"{MUTED_CLASSES}\">{NO_RUN_YET}</span>\
                     <div class=\"text-xs {MUTED_CLASSES} mt-0.5\">asked {} ago</div>",
                    usage::age(start.started)
                ),
                pill(STARTING_PILL, true, PENDING_STATE),
                String::new(),
                escape(&start.game),
                format!("<span class=\"{MUTED_CLASSES}\">{NO_TOURNAMENT}</span>"),
                escape(&start.model),
                String::new(),
                String::new(),
                String::new(),
            ]);
        }
    }

    rows.extend(runs.iter().map(|run| run.row(&registry)));

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
        agent_fields(&registry, "", selection.agent("", ["", DEFAULT_THINKING])),
        select("game", "game", &games, selection.get("game", DEFAULT_GAME)),
        agent_fields(
            &registry,
            crate::serve::ANALYST_PREFIX,
            selection.agent(
                crate::serve::ANALYST_PREFIX,
                [default_analyst(&registry), DEFAULT_ANALYST_THINKING]
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
            [default_analyst(&registry), DEFAULT_ANALYST_THINKING]
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

/// The generated text of a live run, filled by the script off the bus.
fn chat_panel(name: &str, agent: &str) -> String {
    format!(
        "<p class=\"{THINKING_CLASSES}\"><span class=\"{SPINNER_CLASSES}\"></span> {} {THINKING_NOTE}</p>\
         <div class=\"{CHAT_CLASSES}\"><span data-chat=\"{}\"></span>\
         <span class=\"{CARET_CLASSES}\">{CARET}</span></div>",
        escape(agent),
        escape(name)
    )
}

/// The agent the analysis settings offer preselected.
fn default_analyst(registry: &registry::Registry) -> &str {
    registry
        .analyst()
        .map(|alias| alias.name.as_str())
        .unwrap_or_default()
}

/// The dropdowns choosing an agent, the way one is chosen everywhere: the
/// agent by its name in the registry and the thinking level, named under
/// `prefix` in the form, with `selected` marked.
fn agent_fields(registry: &registry::Registry, prefix: &str, selected: [&str; 2]) -> String {
    let [agent_field, thinking_field] = crate::serve::AGENT_FIELDS;
    let [agent, thinking] = selected;

    let configuration = |alias: &registry::Alias| match &alias.backend {
        Some(backend) => format!("{} via {backend}", alias.agent().label()),
        None => alias.agent().label(),
    };
    let row = |alias: &registry::Alias| {
        format!(
            "{}<span class=\"{PICKER_LINES_CLASSES}\"><span class=\"{MONO_CLASSES}\">{}</span>\
             <span class=\"{PICKER_CONFIGURATION_CLASSES}\">{}</span></span>",
            avatar(&alias.agent(), AVATAR_CLASSES),
            escape(&alias.name),
            escape(&configuration(alias))
        )
    };
    let mut levels = vec![""];
    levels.extend(registry::THINKING_LEVELS);

    let agent = match registry.alias(agent).or(registry.agents.first()) {
        None => format!(
            "<span class=\"grow basis-44 {CONTROL_HEIGHT} flex items-center {NOTE_CLASSES}\">{NO_AGENTS_NOTE}</span>"
        ),
        Some(chosen) => {
            let rows: String = registry
                .agents
                .iter()
                .map(|alias| {
                    format!(
                        "<label class=\"{PICKER_ROW_CLASSES}\"><input type=\"radio\" name=\"{prefix}{agent_field}\" value=\"{}\" class=\"sr-only\"{}>{}</label>",
                        escape(&alias.name),
                        checked(alias.name == chosen.name),
                        row(alias)
                    )
                })
                .collect();
            format!(
                "<span class=\"{PICKER_CLASSES}\"><span class=\"{LABEL_CLASSES}\">{agent_field}</span>\
                 <details data-picker><summary class=\"{FIELD_CLASSES} {CONTROL_HEIGHT} {PICKER_FACE_CLASSES}\"><span data-chosen class=\"flex items-center gap-2 min-w-0 grow\">{}</span>{}</summary>\
                 <div class=\"{PICKER_LIST_CLASSES}\">\
                 <input data-filter type=\"text\" autocomplete=\"off\" placeholder=\"{PICKER_FILTER_NOTE}\" class=\"{PICKER_FILTER_CLASSES}\">\
                 {rows}</div></details></span>",
                row(chosen),
                chevron("h-4 w-4 shrink-0 text-neutral-500")
            )
        }
    };

    format!(
        "{agent}{}",
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
    let options: String = options
        .iter()
        .map(|text| option(text, "", *text == selected))
        .collect();

    select_of(name, label, &options)
}

/// A dropdown named `name` under `label`, holding the rendered `options`.
fn select_of(name: &str, label: &str, options: &str) -> String {
    format!(
        "<label class=\"grow basis-44\"><span class=\"{LABEL_CLASSES}\">{label}</span>\
         <select class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" name=\"{name}\">{options}</select></label>"
    )
}

/// One option of a dropdown, with `attributes` of its own.
fn option(text: &str, attributes: &str, selected: bool) -> String {
    let marked = if selected { " selected" } else { "" };
    format!("<option{marked}{attributes}>{}</option>", escape(text))
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
        .map(|metrics| format!("served {}", metrics.served_models.join(" ")))
        .unwrap_or_default();
    let context = entry
        .run
        .context_window
        .map(|window| format!("{window} context tokens"));
    let registry = registry::load()?;
    let agent = entry.run.agent();
    let title = entry.agent_title(&registry);
    let mut facts = vec![
        tile(
            "agent",
            &format!(
                "<span class=\"flex items-center gap-2\">{}<span class=\"truncate\">{}</span></span>",
                avatar(&agent, AGENT_TILE_AVATAR_CLASSES),
                match registry.alias(&title) {
                    Some(_) => agent_link(&title),
                    None => escape(&title),
                }
            ),
            &joined(&[
                &entry.run.harness,
                entry.run.thinking.as_deref().unwrap_or_default(),
                &entry.run.harness_version,
            ]),
            TILE_TEXT_CLASSES,
        ),
        tile(
            "game",
            &escape(&entry.run.game),
            &joined(&[&entry.run.game_version, &entry.run.architecture]),
            TILE_TEXT_CLASSES,
        ),
        tile(
            "model",
            &escape(&entry.run.model),
            &joined(&[
                &entry.run.backend,
                context.as_deref().unwrap_or_default(),
                &served,
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
    // One grid, so the figures follow the facts without a ragged row between them.
    facts.extend([
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
        tile(
            "compactions",
            &match entry.run.compactions {
                Some(compactions) => compactions.to_string(),
                None => placeholder(if entry.live {
                    AFTER_THE_RUN
                } else {
                    NOT_RECORDED
                }),
            },
            "",
            TILE_VALUE_CLASSES,
        ),
    ]);
    body.push_str(&tiles(&facts));

    if entry.live {
        body.push_str(&chat_panel(name, &title));
    }

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
                &["#seconds", "#bytes", "*points", "", "file"],
                rows,
                None,
            ));
        }
    }

    let pushes = attempt_rows(&entry.attempts);
    if !pushes.is_empty() {
        body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">pushes</p>"));
        body.push_str(&table(&["#seconds", "state", "*reason"], pushes, None));
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

    Ok(page(RUNS_HEADING, &body))
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
                "game", "model", "harness", "#runs", "#passed", "*best", "#seconds",
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
    let passed = played.iter().filter(|run| run.passed()).count() as u64;
    let best = played
        .iter()
        .filter_map(|run| run.record.as_ref().map(|record| (*run, record)))
        .max_by_key(|(_, record)| (record.points, record.seconds));

    format!(
        "<details class=\"{GAME_CARD_CLASSES}\" data-fold=\"task-{}\">{}\
         <div class=\"{GAME_BODY_CLASSES}\">{}</div></details>",
        escape(game),
        game_face(game, played.len() as u64, passed, best),
        tasks
            .iter()
            .filter(|text| !text.is_empty())
            .map(|text| ava_markdown::render(text))
            .collect::<Vec<String>>()
            .join(TASK_SEPARATOR)
    )
}

/// The face of a card: the name and its turns, then the cover beside the
/// runs, the image the game plays on and the record with its holder.
fn game_face(
    game: &str,
    runs: u64,
    passed: u64,
    best: Option<(&RunEntry, &runs::Entry)>,
) -> String {
    let image = match ava_game::find(game).and_then(|found| found.image()) {
        Some(image) => format!(
            "<span class=\"{FACT_VALUE_CLASSES}\">{}</span>",
            escape(image)
        ),
        None => format!("<span class=\"{FACT_VALUE_CLASSES}\">{BASE_IMAGE}</span>"),
    };

    let (record, holder) = match best {
        Some((run, entry)) => match entry.points {
            Some(points) => (
                points_meter(points),
                format!("{} \u{00b7} {}", escape(&run.run.model), run.agent()),
            ),
            None => (words(UNRANKED), String::new()),
        },
        None if passed > 0 => (words(NOT_KEPT), String::new()),
        None => (words(NO_ENTRY), String::new()),
    };

    format!(
        "<summary class=\"{GAME_FACE_CLASSES}\">\
         <span class=\"flex items-center gap-3\">\
         <span class=\"{GAME_NAME_CLASSES}\">{}</span>{}<span class=\"flex-1\"></span>\
         {}\
         </span>\
         <span class=\"flex items-start gap-6 mt-4\">{}\
         <span class=\"{FACTS_CLASSES}\">{}{}{}<span class=\"{HOLDER_CLASSES}\">{holder}</span></span>\
         </span>\
         </summary>",
        escape(game),
        turn_badges(game),
        chevron(CHEVRON_CLASSES),
        cover(game, best),
        fact(
            "runs",
            &format!("<span class=\"{FACT_VALUE_CLASSES}\">{runs}</span>")
        ),
        fact("image", &image),
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

/// A value of a card face that is words rather than a number or a name.
fn words(value: &str) -> String {
    format!("<span class=\"{FACT_WORDS_CLASSES}\">{value}</span>")
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
    let registry = registry::load()?;
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
            &registry,
            crate::serve::ANALYST_PREFIX,
            selection.agent(
                crate::serve::ANALYST_PREFIX,
                [default_analyst(&registry), DEFAULT_ANALYST_THINKING]
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
            &["name", "state", "game", "#seats", "#rounds", "*seconds", "#combats"],
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

    // The game as the games page shows it, over the runs of this tournament,
    // and beside it the settings of the tournament.
    let runs = collect_runs()?;
    let played: Vec<&RunEntry> =
        runs.iter()
            .filter(|run| {
                run.run.finished_seconds.is_some()
                    && run.placement.as_ref().is_some_and(|placement| {
                        placement.tournament == name && placement.turn == 0
                    })
            })
            .collect();
    body.push_str(&format!(
        "<div data-refresh=\"about\" class=\"{ABOUT_GRID_CLASSES}\">{}{}</div>",
        game_card(&record.game, &played),
        tournament_card(&record)
    ));

    // The seats with their standings: one table in seat order, the columns of
    // the cross table being seats, the ratings blank until a round finished.
    let removable = !record.played() && !playing;
    let rated = record.finished_rounds().next().is_some();
    let labels: Vec<String> = record.seats.iter().map(|seat| seat.agent.label()).collect();
    let mut labeled = Vec::new();
    for round in record.finished_rounds() {
        labeled.extend(label_pairings(
            &labels,
            &tournament::pairings(&record, round)?,
        ));
    }
    let standings = standings(&labeled);
    let cells = pairing_cells(&record)?;
    let mut seat_rows: Vec<(f64, Vec<String>)> = record
        .seats
        .iter()
        .enumerate()
        .map(|(seat, setup)| {
            let standing = standings
                .iter()
                .find(|standing| standing.agent == labels[seat])
                .filter(|_| rated);
            let score = standing.and_then(|standing| standing.rounds.score());
            let mut row = vec![(seat + 1).to_string()];
            row.extend(agent_cells(&registry, &setup.agent));
            row.extend([
                agent_label(&setup.agent.harness, setup.thinking.as_deref().unwrap_or("")),
                agent_label(&setup.agent.model, setup.backend.as_deref().unwrap_or("")),
            ]);
            row.extend(cells[seat].iter().cloned());
            row.extend([
                standing
                    .map(|standing| tally_label(&standing.fights))
                    .unwrap_or_default(),
                score.map(|score| format!("{score:.2}")).unwrap_or_default(),
                standing
                    .map(|standing| rating_label(standing.elo))
                    .unwrap_or_default(),
                standing
                    .map(|standing| rating_label(standing.bradley_terry))
                    .unwrap_or_default(),
                if removable {
                    format!(
                        "<form method=\"post\" action=\"/tournament/{}/unseat\"><input type=\"hidden\" name=\"seat\" value=\"{seat}\"><button class=\"{STOP_CLASSES}\">remove</button></form>",
                        escape(name)
                    )
                } else {
                    String::new()
                },
            ]);
            (score.unwrap_or(UNRATED_SCORE), row)
        })
        .collect();
    // Seat order until a round is in, the best score on top from then on.
    if rated {
        seat_rows.sort_by(|left, right| right.0.total_cmp(&left.0));
    }
    let seat_rows: Vec<Vec<String>> = seat_rows.into_iter().map(|(_, row)| row).collect();
    let mut headers: Vec<String> = vec![
        "#seat".to_string(),
        String::new(),
        "agent".to_string(),
        "harness|the harness with the thinking level it was asked for".to_string(),
        "*model|the model with the backend serving it".to_string(),
    ];
    headers.extend((1..=record.seats.len()).map(|seat| {
        format!(
            "^{seat}|the row's rounds against seat {seat} over the finished rounds as won-drawn-lost, the rounds behind the hover"
        )
    }));
    let score_column = headers.len() + STANDINGS_SCORE_COLUMN;
    headers.extend(STANDINGS_HEADERS.map(str::to_string));
    let headers: Vec<&str> = headers.iter().map(String::as_str).collect();
    body.push_str(&format!(
        "<div data-refresh=\"lobby\"><p class=\"{TITLE_CLASSES}\">{}</p>{}</div>",
        explained(
            "standings",
            "the seats of the tournament, joining between rounds and fixed once a round was played, their rounds against each other over the finished rounds, and their ratings over the matches between different agents, a harness on a model"
        ),
        sorted_table(
            STANDINGS_TABLE,
            rated.then_some(score_column),
            &headers,
            seat_rows,
            Some(NO_SEATS_NOTE)
        )
    ));
    if removable {
        body.push_str(&format!(
            "<form method=\"post\" action=\"/tournament/{}/seat\" class=\"{CARD_CLASSES} border-t-0 rounded-t-none p-4 flex flex-wrap items-end gap-4\">\
             {}<button class=\"{BUTTON_CLASSES} {CONTROL_HEIGHT}\">seat</button></form>",
            escape(name),
            agent_fields(&registry, "", selection.agent("", ["", DEFAULT_THINKING])),
        ));
    }

    body.push_str("<div data-refresh=\"rounds\">");

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

    Ok(page(TOURNAMENTS_HEADING, &body))
}

/// The tournament in the shape of a game card: the numbered seats on a ring
/// with a line for every pairing as its cover, and the pairing scheme, the
/// seconds of a run and the analyst as its facts.
fn tournament_card(record: &ava_wire::Tournament) -> String {
    let seats = record.seats.len();
    let ring = if seats == 0 {
        format!("<span class=\"{COVER_EMPTY_CLASSES}\"></span>")
    } else {
        format!(
            "<span class=\"{COVER_CLASSES}\">{}</span>",
            pairing_ring(seats)
        )
    };

    let run = format!(
        "<span class=\"{FACT_VALUE_CLASSES}\">{}s</span>",
        record.limit_seconds
    );
    let analyst = match &record.analyst {
        Some(analyst) => format!(
            "<span class=\"{FACT_TEXT_CLASSES}\"><span class=\"{FACT_WORDS_CLASSES} truncate\">{}</span><span class=\"{FACT_VALUE_CLASSES}\">{}s</span></span>",
            escape(&analyst.label()),
            record.analyst_seconds
        ),
        None => words(NO_ANALYST),
    };

    format!(
        "<div class=\"{CARD_CLASSES} p-4 h-full\">\
         <span class=\"flex items-center gap-3\"><span class=\"{GAME_NAME_CLASSES}\">{}</span></span>\
         <span class=\"flex items-start gap-6 mt-4\">{ring}<span class=\"{FACTS_CLASSES}\">{}{}{}</span></span>\
         </div>",
        SETTINGS_TITLE,
        fact("type", &words(&escape(&record.pairing))),
        fact("run", &run),
        fact("analyst", &analyst),
    )
}

/// The `seats` on a ring with a line for every pairing of the round robin,
/// numbered the way the standings and the round graph count them.
fn pairing_ring(seats: usize) -> String {
    let center = RING_SIDE / 2.0;
    let point = |seat: usize| {
        let angle =
            std::f64::consts::TAU * seat as f64 / seats as f64 - std::f64::consts::FRAC_PI_2;
        (
            center + RING_RADIUS * angle.cos(),
            center + RING_RADIUS * angle.sin(),
        )
    };

    let mut svg = format!(
        "<svg class=\"{COVER_ART_CLASSES}\" viewBox=\"0 0 {RING_SIDE} {RING_SIDE}\" stroke-width=\"{RING_STROKE_WIDTH}\">"
    );
    for (first, second) in ava_game::scoring::round_robin(seats) {
        let (from_x, from_y) = point(first);
        let (to_x, to_y) = point(second);
        svg.push_str(&format!(
            "<line x1=\"{from_x:.1}\" y1=\"{from_y:.1}\" x2=\"{to_x:.1}\" y2=\"{to_y:.1}\" class=\"{RING_EDGE_CLASSES}\"/>"
        ));
    }
    for seat in 0..seats {
        let (x, y) = point(seat);
        if seats > RING_NUMBERED_SEATS {
            svg.push_str(&format!(
                "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{RING_DOT_RADIUS}\" class=\"{RING_DOT_CLASSES}\"/>"
            ));
            continue;
        }
        svg.push_str(&format!(
            "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"{RING_SEAT_RADIUS}\" class=\"{RING_SEAT_CLASSES}\"/>\
             <text x=\"{x:.1}\" y=\"{y:.1}\" text-anchor=\"middle\" dominant-baseline=\"central\" font-size=\"{RING_NUMBER_SIZE}\" class=\"{RING_NUMBER_CLASSES}\">{}</text>",
            seat + 1
        ));
    }
    svg.push_str("</svg>");
    svg
}

/// The runs of a round as the graph the tournament walks: a column per turn,
/// a row per seat, every run a node linking its page with its state, and an
/// edge from every entry a run got as its input to that run. Every seat and
/// turn not reached yet is drawn faded, with the edges the game will ask for
/// dashed, so the whole round shows and the part played stands out. While
/// the round is `live`, a run named but not started shows as queued.
fn round_graph(
    record: &ava_wire::Tournament,
    round: &ava_wire::Round,
    game: Option<&dyn ava_game::Game>,
    running: &[String],
    live: bool,
) -> String {
    /// An edge the game will ask for once a turn starts, by seat and turn.
    struct Planned {
        from: (usize, usize),
        to: (usize, usize),
        name: String,
    }

    struct Node {
        seat: usize,
        turn: usize,
        /// The run, empty for a node not played yet.
        run: String,
        record: Option<ava_wire::Run>,
        points: Option<u64>,
        x: f64,
        y: f64,
    }

    let seats = record.seats.len();
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

    let turns = game
        .map_or(1, |game| game.turns().len())
        .max(nodes.iter().map(|node| node.turn + 1).max().unwrap_or(1));

    // Every seat and turn without a run yet is a planned node, with the edges
    // the game will ask for once the turn starts.
    let mut planned_edges: Vec<Planned> = Vec::new();
    for turn in 0..turns {
        for seat in 0..seats {
            if nodes
                .iter()
                .any(|node| node.seat == seat && node.turn == turn)
            {
                continue;
            }
            nodes.push(Node {
                seat,
                turn,
                run: String::new(),
                record: None,
                points: None,
                x: 0.0,
                y: 0.0,
            });
            if let Some(game) = game {
                let opponents: Vec<usize> = (0..seats).filter(|other| *other != seat).collect();
                for input in game.inputs(turn, &opponents) {
                    planned_edges.push(Planned {
                        from: (input.seat, input.turn),
                        to: (seat, turn),
                        name: input.name,
                    });
                }
            }
        }
    }

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
    let at = |seat: usize, turn: usize| {
        nodes
            .iter()
            .find(|node| node.seat == seat && node.turn == turn)
            .map(|node| (node.x, node.y))
    };
    let edge = |from: (f64, f64), to: (f64, f64), name: &str, planned: bool| {
        let (from_x, from_y) = (from.0 + GRAPH_NODE_WIDTH, from.1 + GRAPH_NODE_HEIGHT / 2.0);
        let (to_x, to_y) = (to.0, to.1 + GRAPH_NODE_HEIGHT / 2.0);
        let bend = (from_x + to_x) / 2.0;
        let dashed = if planned {
            format!(
                " stroke-dasharray=\"{GRAPH_PLANNED_DASH}\" class=\"{GRAPH_EDGE_CLASSES} {GRAPH_PLANNED_CLASSES}\""
            )
        } else {
            format!(" class=\"{GRAPH_EDGE_CLASSES}\"")
        };
        format!(
            "<path d=\"M{from_x} {from_y} C{bend} {from_y} {bend} {to_y} {to_x} {to_y}\"{dashed} fill=\"none\"><title>{}</title></path>",
            escape(name)
        )
    };

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
            svg.push_str(&edge(
                (source.x, source.y),
                (node.x, node.y),
                &input.name,
                false,
            ));
        }
    }
    for planned in &planned_edges {
        if let (Some(from), Some(to)) = (
            at(planned.from.0, planned.from.1),
            at(planned.to.0, planned.to.1),
        ) {
            svg.push_str(&edge(from, to, &planned.name, true));
        }
    }

    for node in &nodes {
        let planned = node.run.is_empty();
        let live_run = !planned && running.contains(&docker::scorer_container(&node.run));
        let (state, tint, pulsing) = match (&node.record, live_run) {
            _ if planned => (GRAPH_PLANNED_STATE, MUTED_CLASSES, false),
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
            .map(|seat| seat.agent.label())
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
        let body = format!(
            "<title>{title}</title>\
             <rect x=\"{x}\" y=\"{y}\" width=\"{GRAPH_NODE_WIDTH}\" height=\"{GRAPH_NODE_HEIGHT}\" rx=\"6\" class=\"{GRAPH_NODE_CLASSES}\"/>\
             <text x=\"{text_x}\" y=\"{line_one}\" class=\"{GRAPH_LABEL_CLASSES}\">{shown}</text>{points}\
             <text x=\"{text_x}\" y=\"{line_two}\" class=\"{GRAPH_RUN_CLASSES}\">{run}</text>\
             <circle cx=\"{dot_x}\" cy=\"{dot_y}\" r=\"3\" fill=\"currentColor\" class=\"{tint} {dot_class}\"/>\
             <text x=\"{state_x}\" y=\"{line_two}\" text-anchor=\"end\" fill=\"currentColor\" class=\"{GRAPH_STATE_CLASSES} {tint}\">{state}</text>",
            run = escape(&node.run),
            title = escape(&format!("{label}, {state}")),
            x = node.x,
            y = node.y,
            text_x = node.x + GRAPH_TEXT_INSET,
            line_one = node.y + GRAPH_LINE_ONE,
            line_two = node.y + GRAPH_LINE_TWO,
            dot_x = node.x + GRAPH_NODE_WIDTH
                - GRAPH_TEXT_INSET
                - state.len() as f64 * GRAPH_CHARACTER_WIDTH
                - GRAPH_DOT_GAP,
            dot_y = node.y + GRAPH_LINE_TWO - GRAPH_DOT_LIFT,
            state_x = node.x + GRAPH_NODE_WIDTH - GRAPH_TEXT_INSET,
        );
        if planned {
            svg.push_str(&format!("<g class=\"{GRAPH_PLANNED_CLASSES}\">{body}</g>"));
        } else {
            svg.push_str(&format!(
                "<a href=\"/run/{}\">{body}</a>",
                escape(&node.run)
            ));
        }
    }

    svg.push_str("</svg>");
    format!("<div class=\"{CARD_CLASSES} p-4 overflow-x-auto\">{svg}</div>")
}

/// The place of one agent on a leaderboard.
struct Standing {
    agent: String,
    /// The fights against another agent by outcome, from the agent's view: a
    /// fight with more rounds won than lost is won.
    fights: ava_wire::Tally,
    /// The rounds across those fights, from the agent's view, whose share
    /// won is the score.
    rounds: ava_wire::Tally,
    elo: Option<f64>,
    bradley_terry: Option<f64>,
}

/// One pairing between the agents its seats hold, by their labels.
struct Labeled {
    first: String,
    second: String,
    /// The second it was fought at, the order Elo walks.
    seconds: u64,
    tally: ava_wire::Tally,
}

/// `pairings` between the seats `labels` name, in the order given.
fn label_pairings(labels: &[String], pairings: &[ava_wire::Pairing]) -> Vec<Labeled> {
    pairings
        .iter()
        .filter_map(|pairing| {
            Some(Labeled {
                first: labels.get(pairing.first)?.clone(),
                second: labels.get(pairing.second)?.clone(),
                seconds: pairing.seconds,
                tally: pairing.tally,
            })
        })
        .collect()
}

/// The standings over `labeled`, every agent that met another, rated over
/// the matches between different agents by the second they were fought,
/// Bradley-Terry first.
fn standings(labeled: &[Labeled]) -> Vec<Standing> {
    let mut labeled: Vec<&Labeled> = labeled.iter().collect();
    labeled.sort_by_key(|pairing| pairing.seconds);
    let matches: Vec<ava_game::scoring::Match> = labeled
        .iter()
        .filter(|pairing| pairing.first != pairing.second)
        .filter_map(|pairing| {
            Some(ava_game::scoring::Match {
                first: pairing.first.clone(),
                second: pairing.second.clone(),
                score: pairing.tally.score()?,
            })
        })
        .collect();
    let elo = ava_game::scoring::Elo.leaderboard(&matches);
    let bradley_terry = ava_game::scoring::BradleyTerry.leaderboard(&matches);
    let rating = |leaderboard: &[ava_game::scoring::Rating], agent: &str| {
        leaderboard
            .iter()
            .find(|rating| rating.agent == agent)
            .map(|rating| rating.rating)
    };

    let mut agents: Vec<String> = Vec::new();
    for pairing in &labeled {
        for agent in [&pairing.first, &pairing.second] {
            if !agents.contains(agent) {
                agents.push(agent.clone());
            }
        }
    }

    let mut standings: Vec<Standing> = agents
        .into_iter()
        .map(|agent| {
            let mut fights = ava_wire::Tally::default();
            let mut rounds = ava_wire::Tally::default();
            for pairing in &labeled {
                if pairing.first == pairing.second || pairing.tally.rounds() == 0 {
                    continue;
                }
                let view = if pairing.first == agent {
                    pairing.tally
                } else if pairing.second == agent {
                    mirrored(&pairing.tally)
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

    standings
}

/// The `tally` from the view of its second side.
fn mirrored(tally: &ava_wire::Tally) -> ava_wire::Tally {
    ava_wire::Tally {
        won: tally.lost,
        drawn: tally.drawn,
        lost: tally.won,
    }
}

/// A tally as won-drawn-lost.
fn tally_label(tally: &ava_wire::Tally) -> String {
    format!("{}-{}-{}", tally.won, tally.drawn, tally.lost)
}

/// A rating rounded to the point, or nothing for an agent without matches.
fn rating_label(rating: Option<f64>) -> String {
    rating
        .map(|rating| format!("{}", rating.round() as i64))
        .unwrap_or_default()
}

/// The cells of the standings, by seat row and seat column: the row's rounds
/// against the column over the finished rounds, tinted by who came out ahead,
/// `none` where nothing was counted, the rounds behind the hover with their
/// reasons. A pairing recorded the other way round is read mirrored.
fn pairing_cells(record: &ava_wire::Tournament) -> std::io::Result<Vec<Vec<String>>> {
    #[derive(Default)]
    struct Met {
        tally: ava_wire::Tally,
        rounds: Vec<String>,
    }

    let seats = record.seats.len();
    let mut met: Vec<Vec<Option<Met>>> = (0..seats)
        .map(|_| (0..seats).map(|_| None).collect())
        .collect();
    for (index, round) in record.rounds.iter().enumerate() {
        if round.finished_seconds.is_none() {
            continue;
        }
        for pairing in tournament::pairings(record, round)? {
            for (row, column, view) in [
                (pairing.first, pairing.second, pairing.tally),
                (pairing.second, pairing.first, mirrored(&pairing.tally)),
            ] {
                if row == column || row >= seats || column >= seats {
                    continue;
                }
                let cell = met[row][column].get_or_insert_with(Met::default);
                cell.tally.won += view.won;
                cell.tally.drawn += view.drawn;
                cell.tally.lost += view.lost;
                cell.rounds.push(match &pairing.reason {
                    Some(reason) => {
                        format!("round {}: {}, {reason}", index + 1, tally_label(&view))
                    }
                    None => format!("round {}: {}", index + 1, tally_label(&view)),
                });
            }
        }
    }

    Ok(met
        .into_iter()
        .enumerate()
        .map(|(row, cells)| {
            cells
                .into_iter()
                .enumerate()
                .map(|(column, cell)| {
                    if row == column {
                        return format!("<span class=\"{MUTED_CLASSES}\">\u{00b7}</span>");
                    }
                    let Some(cell) = cell else {
                        return String::new();
                    };
                    let label = if cell.tally.rounds() == 0 {
                        format!("<span class=\"{MUTED_CLASSES}\">none</span>")
                    } else {
                        format!(
                            "<span class=\"{MONO_CLASSES} {}\">{}</span>",
                            tint(&cell.tally),
                            tally_label(&cell.tally)
                        )
                    };
                    explained(&label, &cell.rounds.join(" \u{00b7} "))
                })
                .collect()
        })
        .collect())
}

/// The colour of a tally from the view of its first side.
fn tint(tally: &ava_wire::Tally) -> &'static str {
    match tally.won.cmp(&tally.lost) {
        std::cmp::Ordering::Greater => AHEAD_CLASSES,
        std::cmp::Ordering::Less => BEHIND_CLASSES,
        std::cmp::Ordering::Equal => LEVEL_CLASSES,
    }
}

/// The agents named in the registry, and the form naming one.
pub(crate) fn agents_page(notice: &Notice, selection: &Selection) -> std::io::Result<String> {
    let registry = registry::load()?;
    let mut body = format!(
        "<p class=\"{FIRST_TITLE_CLASSES}\">new agent</p>{}",
        alias_panel(&registry, selection, None)
    );
    body.push_str(&notice.render());
    body.push_str(&format!(
        "<p class=\"{TITLE_CLASSES}\">{}</p>",
        explained(
            "agents",
            "an agent is a harness on a model, named in agents.json"
        )
    ));
    let rows = registry
        .agents
        .iter()
        .map(|alias| {
            let mut row = named_cells(&alias.agent(), &alias.name).to_vec();
            row.extend([
                escape(&alias.harness),
                escape(&alias.model),
                escape(alias.backend.as_deref().unwrap_or_default()),
                if alias.analyst {
                    pill(ANALYZED_PILL, false, "analyst")
                } else {
                    String::new()
                },
                alias_actions(alias),
            ]);
            row
        })
        .collect();
    body.push_str(&sorted_table(
        AGENTS_TABLE,
        None,
        &[
            "",
            "agent",
            "harness",
            "*model",
            "backend|the backend serving the model, the first route of the model when empty",
            "|the agent analyzing runs unless another is chosen",
            "",
        ],
        rows,
        Some(NO_AGENTS_ROW_NOTE),
    ));

    Ok(page(AGENTS_HEADING, &body))
}

/// The page of one agent: what it pairs, how its runs went, who it met in the
/// tournaments, the runs themselves and the form changing it.
pub(crate) fn agent_page(
    name: &str,
    notice: &Notice,
    selection: &Selection,
) -> std::io::Result<String> {
    let registry = registry::load()?;
    let alias = registry
        .alias(name)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no agent named `{name}`"),
            )
        })?
        .clone();
    let agent = alias.agent();
    let runs = collect_runs()?;
    let played: Vec<&RunEntry> = runs.iter().filter(|entry| entry.by(&alias)).collect();

    let mut body = format!(
        "<div class=\"flex items-center gap-3\">{}\
         <span class=\"text-lg font-semibold text-neutral-100 {MONO_CLASSES}\">{}</span>{}</div>",
        avatar(&agent, PROFILE_AVATAR_CLASSES),
        escape(&alias.name),
        if alias.analyst {
            pill(ANALYZED_PILL, false, "analyst")
        } else {
            String::new()
        }
    );
    body.push_str(&notice.render());
    body.push_str("<div data-refresh=\"agent\">");

    let live = played.iter().filter(|entry| entry.live).count();
    let finished = played
        .iter()
        .filter(|entry| entry.run.finished_seconds.is_some())
        .count();
    let passed = played.iter().filter(|entry| entry.passed()).count();
    let tokens: u64 = played
        .iter()
        .filter_map(|entry| entry.metrics.as_ref())
        .map(|metrics| metrics.output_tokens)
        .sum();
    let spent: u64 = played
        .iter()
        .filter_map(|entry| entry.run.wall_seconds())
        .sum();
    body.push_str(&tiles(&[
        tile("harness", &escape(&alias.harness), "", TILE_TEXT_CLASSES),
        tile(
            "model",
            &escape(&alias.model),
            &escape(
                &registry
                    .route(&alias.harness, &alias.model, alias.backend.as_deref())
                    .map(|(_, backend)| backend.name.clone())
                    .unwrap_or_default(),
            ),
            TILE_TEXT_CLASSES,
        ),
        tile(
            "runs",
            &played.len().to_string(),
            &if live > 0 {
                format!("{live} live")
            } else {
                String::new()
            },
            TILE_VALUE_CLASSES,
        ),
        tile(
            "passed",
            &passed.to_string(),
            &format!("of {finished} finished"),
            TILE_VALUE_CLASSES,
        ),
        tile("output tokens", &tokens.to_string(), "", TILE_VALUE_CLASSES),
        tile("time played", &usage::span(spent), "", TILE_VALUE_CLASSES),
    ]));

    if !played.is_empty() {
        body.push_str(&format!(
            "<p class=\"{TITLE_CLASSES}\">{}</p>{}",
            explained("form", "the last runs, oldest first"),
            form_strip(&played)
        ));
    }

    body.push_str(&format!(
        "<p class=\"{TITLE_CLASSES}\">{}</p>{}",
        explained(
            "rivals",
            "the agents this one was paired against in a tournament"
        ),
        sorted_table(
            RIVALS_TABLE,
            Some(RIVALS_FOUGHT_COLUMN),
            &RIVALS_HEADERS,
            rivals(&registry, &agent)?,
            Some(NO_RIVALS_NOTE),
        )
    ));

    // The agent is the page, so the runs table drops its column.
    let rows = played
        .iter()
        .map(|entry| entry.row(&registry)[1..].to_vec())
        .collect();
    body.push_str(&format!(
        "<p class=\"{TITLE_CLASSES}\">{RUNS_HEADING}</p>{}",
        table(&RUN_HEADERS[1..], rows, Some(NO_AGENT_RUNS_NOTE))
    ));
    body.push_str("</div>");

    // The form stays outside the refreshed region, so what is typed into it
    // survives the refresh.
    body.push_str(&format!(
        "<p class=\"{TITLE_CLASSES}\">{SETTINGS_TITLE}</p>{}",
        alias_panel(&registry, selection, Some(&alias))
    ));

    Ok(page(AGENTS_HEADING, &body))
}

/// The last runs as one square each, oldest first, tinted by outcome and
/// leading to the run.
fn form_strip(played: &[&RunEntry]) -> String {
    let marks: String = played
        .iter()
        .take(FORM_RUNS)
        .rev()
        .map(|entry| {
            let (tint, state) = if entry.live {
                (FORM_LIVE, "live")
            } else if entry.passed() {
                (FORM_PASSED, "passed")
            } else if entry.run.finished_seconds.is_some() {
                (FORM_FAILED, "failed")
            } else {
                (FORM_BROKEN, UNFINISHED)
            };
            format!(
                "<a class=\"{FORM_MARK_CLASSES} {tint}\" href=\"/run/{run}\" title=\"{run} \u{00b7} {game} \u{00b7} {state}\"></a>",
                run = escape(&entry.name),
                game = escape(&entry.run.game)
            )
        })
        .collect();

    format!("<div class=\"{FORM_CLASSES}\">{marks}</div>")
}

/// The rows of the agents `agent` was paired against in a tournament, from its
/// view, over the finished rounds of every tournament.
fn rivals(
    registry: &registry::Registry,
    agent: &ava_wire::Agent,
) -> std::io::Result<Vec<Vec<String>>> {
    struct Met {
        agent: ava_wire::Agent,
        tally: ava_wire::Tally,
        /// The pairings the two played.
        fought: u64,
    }

    let mut met: Vec<Met> = Vec::new();
    for record in tournament::list()? {
        for round in record.finished_rounds() {
            for pairing in tournament::pairings(&record, round)? {
                let (Some(first), Some(second)) = (
                    record.seats.get(pairing.first),
                    record.seats.get(pairing.second),
                ) else {
                    continue;
                };
                if pairing.tally.rounds() == 0 {
                    continue;
                }
                let (other, view) = if first.agent == *agent && second.agent != *agent {
                    (&second.agent, pairing.tally)
                } else if second.agent == *agent && first.agent != *agent {
                    (&first.agent, mirrored(&pairing.tally))
                } else {
                    continue;
                };

                let seen = match met.iter().position(|seen| seen.agent == *other) {
                    Some(index) => &mut met[index],
                    None => {
                        met.push(Met {
                            agent: other.clone(),
                            tally: ava_wire::Tally::default(),
                            fought: 0,
                        });
                        met.last_mut().expect("just pushed")
                    }
                };
                seen.tally.won += view.won;
                seen.tally.drawn += view.drawn;
                seen.tally.lost += view.lost;
                seen.fought += 1;
            }
        }
    }

    // The most fought rival first, since one round decides nothing.
    met.sort_by_key(|seen| std::cmp::Reverse(seen.fought));

    Ok(met
        .iter()
        .map(|seen| {
            let mut row = agent_cells(registry, &seen.agent).to_vec();
            row.extend([
                format!(
                    "<span class=\"{MONO_CLASSES} {}\">{}</span>",
                    tint(&seen.tally),
                    tally_label(&seen.tally)
                ),
                seen.fought.to_string(),
                seen.tally
                    .score()
                    .map(|score| format!("{score:.2}"))
                    .unwrap_or_default(),
            ]);
            row
        })
        .collect())
}

/// The form naming an agent in the registry, or changing `editing`, with what
/// was submitted or that agent preselected.
fn alias_panel(
    registry: &registry::Registry,
    selection: &Selection,
    editing: Option<&registry::Alias>,
) -> String {
    let fields = crate::serve::ALIAS_FIELDS;
    let [name_field, harness_field, model_field, analyst_field] = fields;
    let route = |alias: &registry::Alias| {
        let backend = registry
            .route(&alias.harness, &alias.model, alias.backend.as_deref())
            .map(|(_, backend)| backend.name.clone())
            .unwrap_or_default();
        format!("{}{}{backend}", alias.model, registry::ROUTE_SEPARATOR)
    };
    let (action, button, defaults) = match editing {
        Some(alias) => (
            format!("/agents/{}/edit", escape(&alias.name)),
            "save",
            [
                alias.name.clone(),
                alias.harness.clone(),
                route(alias),
                if alias.analyst { "on" } else { "" }.to_string(),
            ],
        ),
        None => (
            "/agents/create".to_string(),
            "add",
            [String::new(), String::new(), String::new(), String::new()],
        ),
    };
    let chosen: Vec<&str> = fields
        .iter()
        .zip(&defaults)
        .map(|(field, default)| selection.get(field, default))
        .collect();
    let [name, harness, model, analyst] = chosen[..] else {
        unreachable!("one choice per field")
    };

    // The harness option carries the services it speaks and every route
    // option the service of its backend, so the page script offers only the
    // routes the harness speaks.
    let harnesses: String = registry
        .harnesses
        .iter()
        .map(|known| {
            let services: Vec<&str> = known
                .services
                .iter()
                .map(|service| service.name())
                .collect();
            option(
                &known.name,
                &format!(" data-services=\"{}\"", escape(&services.join(" "))),
                known.name == harness,
            )
        })
        .collect();
    let routes: String = registry
        .models
        .iter()
        .flat_map(|known| {
            known.routes.iter().map(move |served| {
                let text = if known.routes.len() > 1 {
                    format!("{} via {}", known.name, served.backend)
                } else {
                    known.name.clone()
                };
                let value = format!(
                    "{}{}{}",
                    known.name,
                    registry::ROUTE_SEPARATOR,
                    served.backend
                );
                let service = registry
                    .backend(&served.backend)
                    .map(|backend| backend.service.name())
                    .unwrap_or_default();
                option(
                    &text,
                    &format!(" value=\"{}\" data-service=\"{service}\"", escape(&value)),
                    value == model,
                )
            })
        })
        .collect();
    format!(
        "<form method=\"post\" action=\"{action}\" class=\"{CARD_CLASSES} p-4 flex flex-wrap items-end gap-4\">\
         <label class=\"grow basis-44\"><span class=\"{LABEL_CLASSES}\">{}</span>\
         <input class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" type=\"text\" name=\"{name_field}\" value=\"{}\" placeholder=\"letters, digits, dashes\" required></label>\
         {}{}\
         <label class=\"{CONTROL_HEIGHT} flex items-center gap-2\">\
         <input type=\"checkbox\" name=\"{analyst_field}\" class=\"h-4 w-4 rounded accent-indigo-500\"{}>\
         <span class=\"{NOTE_CLASSES}\">{}</span></label>\
         <button class=\"{BUTTON_CLASSES} {CONTROL_HEIGHT}\">{button}</button></form>",
        explained(
            name_field,
            "what selects the agent on the command line and in the forms; the records hold the harness and the model, so renaming changes nothing there"
        ),
        escape(name),
        select_of(harness_field, harness_field, &harnesses),
        select_of(
            model_field,
            &explained(
                model_field,
                "the model at the backend serving it, one entry per route of the registry"
            ),
            &routes
        ),
        checked(analyst == "on"),
        explained(
            "analyst",
            "the agent analyzing runs unless another is chosen, one at most; marking this one unmarks the other"
        ),
    )
}

/// The remove button of `alias` on the agents page.
fn alias_actions(alias: &registry::Alias) -> String {
    format!(
        "<form method=\"post\" action=\"/agents/{}/delete\"><button class=\"{STOP_CLASSES}\">remove</button></form>",
        escape(&alias.name)
    )
}

/// Two cells: the avatar of `agent`, and its name in the registry, or the
/// harness on the model when the registry has none for it.
fn agent_cells(registry: &registry::Registry, agent: &ava_wire::Agent) -> [String; 2] {
    match registry.alias_of(agent) {
        Some(alias) => named_cells(agent, &alias.name),
        None => [
            avatar(agent, AVATAR_CLASSES),
            format!(
                "<span class=\"{MONO_CLASSES}\">{}</span>",
                escape(&agent.label())
            ),
        ],
    }
}

/// The avatar of `agent` beside `name`, over the harness and the thinking level.
fn agent_stack(name: String, agent: &ava_wire::Agent, thinking: &str) -> String {
    format!(
        "<span class=\"flex items-center gap-2\">{}{}</span>\
         <div class=\"text-xs {MUTED_CLASSES} mt-0.5\">{} {}</div>",
        avatar(agent, AGENT_TILE_AVATAR_CLASSES),
        escape(&name),
        escape(&agent.harness),
        escape(thinking)
    )
}

/// The avatar of `agent` beside `name`, leading to the page of the agent.
fn named_cells(agent: &ava_wire::Agent, name: &str) -> [String; 2] {
    [avatar(agent, AVATAR_CLASSES), agent_link(name)]
}

/// `name` leading to the page of the agent it names.
fn agent_link(name: &str) -> String {
    format!(
        "<a class=\"{LINK_CLASSES}\" href=\"/agent/{name}\">{name}</a>",
        name = escape(name)
    )
}

/// The name `agent` is registered under, else the harness on the model.
fn agent_name(registry: &registry::Registry, agent: &ava_wire::Agent) -> String {
    registry
        .alias_of(agent)
        .map(|alias| alias.name.clone())
        .unwrap_or_else(|| agent.label())
}

/// The avatar of `agent`: a grid of cells lit by the bits of the hash of its
/// harness and model, mirrored left to right, in a hue the hash picks. The
/// same agent has the same avatar everywhere, whatever it is named.
fn avatar(agent: &ava_wire::Agent, classes: &str) -> String {
    let hash = fnv1a(&[agent.harness.as_bytes(), &[0], agent.model.as_bytes()].concat());
    let hue = (hash >> (AVATAR_SIDE * AVATAR_COLUMNS)) % AVATAR_HUES;
    let mut cells = String::new();
    for row in 0..AVATAR_SIDE {
        for column in 0..AVATAR_COLUMNS {
            if hash >> (row * AVATAR_COLUMNS + column) & 1 == 0 {
                continue;
            }
            for x in [column, AVATAR_SIDE - 1 - column] {
                cells.push_str(&format!(
                    "<rect x=\"{x}\" y=\"{row}\" width=\"1\" height=\"1\"/>"
                ));
            }
        }
    }

    format!(
        "<svg class=\"{classes}\" viewBox=\"0 0 {AVATAR_SIDE} {AVATAR_SIDE}\" shape-rendering=\"crispEdges\" role=\"img\" aria-label=\"{}\">\
         <rect width=\"{AVATAR_SIDE}\" height=\"{AVATAR_SIDE}\" class=\"{AVATAR_GROUND_CLASSES}\"/>\
         <g fill=\"hsl({hue} 60% 55%)\">{cells}</g></svg>",
        escape(&agent.label())
    )
}

/// The 64 bit FNV-1a hash of `bytes`, the same on every toolchain.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
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
            "backend",
            "service",
            "host",
            "key",
            "*state",
            "#runs",
            "#analyses",
            "#requests",
            "#input",
            "#output",
            "#cache read",
            "#cache write",
            "#cost",
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
            "backend",
            "window",
            "*used",
            "left",
            "status",
            "*resets|how far the window has run towards its reset, and when it resets",
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
        &["model", "backend", "*id", "#context", "#max output"],
        model_rows,
        None,
    ));
    body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">harnesses</p>"));
    body.push_str(&table(&["harness", "services"], harness_rows, None));

    if let Some(rows) = images {
        body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">images</p>"));
        body.push_str(&table(&["image", "tag", "size", "created"], rows, None));
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

/// A grid of tiles.
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

/// A table in a card. The markers leading a header set its column: `#`
/// right-aligns numbers, `^` centers, `*` takes a share of the slack, and
/// they combine. Without a `*` the last column takes the slack. Without rows
/// the table shows `empty`, or nothing when there is no note to show.
fn table(headers: &[&str], rows: Vec<Vec<String>>, empty: Option<&str>) -> String {
    render_table(None, None, headers, rows, empty)
}

/// A table its headers sort, arriving sorted by `sorted`, the script keeping
/// the chosen column under `name` across a refresh.
fn sorted_table(
    name: &str,
    sorted: Option<usize>,
    headers: &[&str],
    rows: Vec<Vec<String>>,
    empty: Option<&str>,
) -> String {
    render_table(Some(name), sorted, headers, rows, empty)
}

/// The direction a column sorts in when it is picked.
fn sort_order(alignment: &str) -> &'static str {
    if alignment == NUMERIC_CLASSES {
        DESCENDING_ORDER
    } else {
        ASCENDING_ORDER
    }
}

/// The arrow on the header a table is sorted by.
fn sort_arrow(alignment: &str) -> &'static str {
    if sort_order(alignment) == DESCENDING_ORDER {
        DESCENDING_ARROW
    } else {
        ASCENDING_ARROW
    }
}

fn render_table(
    sortable: Option<&str>,
    sorted: Option<usize>,
    headers: &[&str],
    rows: Vec<Vec<String>>,
    empty: Option<&str>,
) -> String {
    if rows.is_empty() && empty.is_none() {
        return String::new();
    }

    let markers = |header: &str| -> Vec<char> {
        header
            .chars()
            .take_while(|character| MARKERS.contains(character))
            .collect()
    };
    let alignment: Vec<&str> = headers
        .iter()
        .map(|header| {
            let markers = markers(header);
            if markers.contains(&NUMERIC_MARKER) {
                NUMERIC_CLASSES
            } else if markers.contains(&CENTER_MARKER) {
                CENTERED_CLASSES
            } else {
                ""
            }
        })
        .collect();
    let mut slack: Vec<usize> = headers
        .iter()
        .enumerate()
        .filter(|(_, header)| markers(header).contains(&SLACK_MARKER))
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

    // The slack columns are the same width, so a block of packed columns
    // between two of them sits where their contents do not push it.
    let share = format!(" style=\"width:{}%\"", 100 / slack.len().max(1));
    let sorting = match sortable {
        Some(name) => format!(
            " data-sortable=\"{}\"{}",
            escape(name),
            match sorted {
                Some(column) => format!(
                    " data-sorted=\"{column}\" data-order=\"{}\"",
                    sort_order(alignment.get(column).copied().unwrap_or_default())
                ),
                None => String::new(),
            }
        ),
        None => String::new(),
    };
    let mut html = format!(
        "<div class=\"{CARD_CLASSES} overflow-x-auto\"><table class=\"{TABLE_CLASSES}\"{sorting}><thead><tr class=\"{HEADER_ROW_CLASSES}\">"
    );
    for (index, (header, alignment)) in headers.iter().zip(&alignment).enumerate() {
        let align = match *alignment {
            NUMERIC_CLASSES => "text-right",
            CENTERED_CLASSES => "text-center",
            _ => "text-left",
        };
        let classes = column(index);
        let width = if slack.contains(&index) {
            share.as_str()
        } else {
            ""
        };
        let (title, tooltip) = header.split_once(TOOLTIP_SEPARATOR).unwrap_or((header, ""));
        let title = title.trim_start_matches(MARKERS);
        let (sortable_classes, sort, arrow) = match sortable.filter(|_| !title.is_empty()) {
            Some(_) => (
                SORTABLE_HEADER_CLASSES,
                format!(
                    " data-sort=\"{}\"",
                    if *alignment == NUMERIC_CLASSES {
                        NUMERIC_SORT
                    } else {
                        TEXT_SORT
                    }
                ),
                format!(
                    "<span data-arrow class=\"{SORT_ARROW_CLASSES}\">{}</span>",
                    match sorted {
                        Some(column) if column == index => sort_arrow(alignment),
                        _ => "",
                    }
                ),
            ),
            None => ("", String::new(), String::new()),
        };
        html.push_str(&format!(
            "<th class=\"{classes} {HEADER_CLASSES} {align} {sortable_classes}\"{width}{sort}>{}{arrow}</th>",
            explained(title, tooltip)
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
        for (index, (cell, align)) in row.iter().zip(&alignment).enumerate() {
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
    }

    #[test]
    fn an_avatar_is_mirrored_and_the_same_for_the_same_agent() {
        let agent = ava_wire::Agent {
            harness: "pi".to_string(),
            model: "m".to_string(),
        };
        let drawn = super::avatar(&agent, "");
        assert_eq!(drawn, super::avatar(&agent.clone(), ""));

        let cells = drawn.matches("<rect x=").count();
        assert_eq!(cells % 2, 0);
        for x in 0..super::AVATAR_COLUMNS {
            let mirror = super::AVATAR_SIDE - 1 - x;
            assert_eq!(
                drawn.matches(&format!("<rect x=\"{x}\"")).count(),
                drawn.matches(&format!("<rect x=\"{mirror}\"")).count()
            );
        }
    }

    #[test]
    fn no_cover_comes_from_outside_the_games_directory() {
        assert!(super::cover_path("../Cargo.toml").is_none());
        assert!(super::game_cover("no-such-game").is_none());
    }
}
