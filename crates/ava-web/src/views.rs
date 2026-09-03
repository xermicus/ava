//! The pages of the web interface, rendered from the run artifacts on disk,
//! the registry and the games folder.

use ava_run::{docker, process, registry};

const GAMES_DIRECTORY: &str = "games";
const TASK_FILE: &str = "task.md";
const INSTRUCTIONS_FILE: &str = "README.md";

/// Every game scores within this ceiling, which is what makes one point
/// bar comparable across all of them.
const POINT_CEILING: u64 = 10000;

/// How many solving runs a game lists on its standings.
const STANDINGS_LIMIT: usize = 3;

/// What the start panel offers preselected on a fresh page.
const DEFAULT_GAME: &str = "sanity-check";
const DEFAULT_THINKING: &str = "medium";

/// The files of a run the raw file routes hand out, and nothing else.
const RUN_FILES: [&str; 9] = [
    docker::MONITOR_FILE,
    docker::AGENT_LOG,
    docker::SCORE_LOG,
    docker::METADATA_FILE,
    docker::SCORE_FILE,
    docker::ACCESS_LOG,
    docker::ERROR_LOG,
    docker::SCORE_ERROR_LOG,
    docker::VERSION_FILE,
];

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
const RUN_HEADERS: [&str; 10] = [
    "RUN|the run directory under runs/, and how long ago it started",
    "STATE|live or the last call while the run goes, the verdict of its best push once it is over",
    "GAME|the game that was played, and the scorer that graded it",
    "MODEL|the model under test",
    "HARNESS|the harness driving the model, with the thinking level it was asked for",
    "*TIME|seconds spent of the time budget, red once the whole budget is gone",
    "#PUSHES|the pushes to the task branch the scorer graded",
    "#CUT|requests a model answered without ever reporting usage, so the stream was cut short \
     upstream",
    "*POINTS|the best solving push, on the 0 to 10000 scale every game scores within",
    "",
];
const NO_RUNS_NOTE: &str = "no runs yet, start one above";
const NO_LIMITS_NOTE: &str = "no limits reported yet, the first run of a backend brings them";

/// The rolling windows the Anthropic subscription reports, by the header
/// infix naming them.
const ANTHROPIC_WINDOWS: [(&str, &str); 2] = [("5h", "session, 5 hours"), ("7d", "week, 7 days")];
const ANTHROPIC_LIMIT_PREFIX: &str = "anthropic-ratelimit-unified-";

/// The budget of a gateway key, as the gateway names it in its headers.
const GATEWAY_SPEND_HEADER: &str = "x-litellm-key-spend";
const GATEWAY_BUDGET_HEADER: &str = "x-litellm-key-max-budget";

/// The request and token windows a gateway reports, by the header suffix
/// naming them.
const GATEWAY_WINDOWS: [&str; 2] = ["requests", "tokens"];
const GATEWAY_LIMIT_PREFIX: &str = "x-ratelimit-";

const SECONDS_PER_DAY: u64 = 86_400;
const SECONDS_PER_HOUR: u64 = 3_600;
const SECONDS_PER_MINUTE: u64 = 60;

/// A card holds one table or one form, so every block on a page shares the
/// same edges and corners.
const CARD_CLASSES: &str = "rounded-lg border border-neutral-800 bg-neutral-900";

/// Every table spans its card. The columns pack on one gutter, shrunk to
/// their content, and one of them takes the slack, so the columns before it
/// start on the left edge, the ones after it end on the right edge and every
/// gap stays the same width. Without a marked column the last one takes the
/// slack.
const TABLE_CLASSES: &str = "w-full border-collapse";
const PACKED_COLUMN_CLASSES: &str = "w-px whitespace-nowrap px-3 first:pl-4 last:pr-4";
const SLACK_COLUMN_CLASSES: &str = "w-full px-3 first:pl-4 last:pr-4";
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
     font-mono text-neutral-300 whitespace-pre-wrap break-all overflow-x-auto";
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
const SOLVED_PILL: &str = "bg-emerald-500/10 text-emerald-400";
const UNSOLVED_PILL: &str = "bg-orange-500/10 text-orange-400";
const FAILED_PILL: &str = "bg-red-500/10 text-red-400";
const STARTING_PILL: &str = "bg-amber-500/10 text-amber-400";
const NEUTRAL_PILL: &str = "bg-neutral-800 text-neutral-400";

/// The meters: a track, a fill and a mono label.
const METER_TRACK_CLASSES: &str =
    "h-1.5 flex-1 min-w-16 rounded-full bg-neutral-800 overflow-hidden";

/// The labels beside the meters have one width per kind, so the tracks of
/// one column start and end on the same lines.
const POINTS_LABEL_WIDTH: &str = "w-12";
const ELAPSED_LABEL_WIDTH: &str = "w-24";
const USAGE_LABEL_WIDTH: &str = "w-16";
const USAGE_FILL: &str = "bg-amber-500";
const POINTS_FILL: &str = "bg-amber-500";

/// The time meter, tinted by whether the budget held.
const TIME_SPENT_FILL: &str = "bg-red-500";
const TIME_LEFT_FILL: &str = "bg-emerald-500";

/// The tiles summarizing a run, one figure each.
const TILE_CLASSES: &str = "rounded-lg border border-neutral-800 bg-neutral-900 px-4 py-3";
const TILE_LABEL_CLASSES: &str = "text-xs font-medium uppercase tracking-wider text-neutral-500";
const TILE_VALUE_CLASSES: &str =
    "mt-1 text-lg font-semibold text-neutral-100 font-mono tabular-nums";

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

/// What the start panel shows selected, carried through the action redirect
/// so a submission does not reset the form.
#[derive(Default)]
pub(crate) struct Selection {
    pub agent: Option<String>,
    pub model: Option<String>,
    pub game: Option<String>,
    pub thinking: Option<String>,
    pub limit: Option<String>,
    pub parallel: Option<String>,
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
    metadata: serde_json::Value,
    score: Option<serde_json::Value>,
    live: bool,
    /// The wall clock seconds the run actually took, for a finished run.
    wall: Option<u64>,
    /// The newest heartbeat of the run loop, for a live run.
    monitor: Option<serde_json::Value>,
    /// The scored pushes so far: the attempts log once collected, the output
    /// of the scoring container while the run is live.
    attempts: Vec<serde_json::Value>,
}

impl RunEntry {
    fn game(&self) -> &str {
        text(&self.metadata, "game")
    }

    fn model(&self) -> &str {
        text(&self.metadata, "model")
    }

    fn harness(&self) -> &str {
        text(&self.metadata, "agent")
    }

    fn thinking(&self) -> &str {
        text(&self.metadata, "thinking")
    }

    /// The harness with its thinking level, the way an agent is referred to.
    fn agent(&self) -> String {
        agent_label(self.harness(), self.thinking())
    }

    fn started(&self) -> u64 {
        number(&self.metadata, "started_seconds")
    }

    fn limit(&self) -> u64 {
        number(&self.metadata, "limit_seconds")
    }

    fn attempts(&self) -> u64 {
        pointer(self.score.as_ref(), "/attempts/attempts")
    }

    fn points(&self) -> u64 {
        pointer(self.score.as_ref(), "/attempts/points")
    }

    fn seconds(&self) -> u64 {
        pointer(self.score.as_ref(), "/attempts/first_solved_seconds")
    }

    /// Every host the run requested through its proxy, attributing the run to
    /// the backend that served it.
    fn hosts(&self) -> Vec<&str> {
        self.score
            .as_ref()
            .and_then(|score| score.pointer("/metrics/hosts"))
            .and_then(serde_json::Value::as_array)
            .map(|items| items.iter().filter_map(serde_json::Value::as_str).collect())
            .unwrap_or_default()
    }

    /// One aggregated number out of the proxy metrics of the run.
    fn metric(&self, key: &str) -> u64 {
        pointer(self.score.as_ref(), &format!("/metrics/{key}"))
    }

    /// One aggregated fraction out of the proxy metrics of the run.
    fn metric_float(&self, key: &str) -> f64 {
        self.score
            .as_ref()
            .and_then(|score| score.pointer(&format!("/metrics/{key}")))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
    }

    /// One aggregated text out of the proxy metrics of the run.
    fn metric_text(&self, key: &str) -> &str {
        self.score
            .as_ref()
            .and_then(|score| score.pointer(&format!("/metrics/{key}")))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
    }

    fn solved(&self) -> bool {
        self.score
            .as_ref()
            .and_then(|score| score.pointer("/attempts/solved"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }

    /// Whether the score obeys the point ceiling; one scored before the
    /// ceiling existed does not compete on the standings.
    fn comparable(&self) -> bool {
        self.points() <= POINT_CEILING
    }

    /// The run name as a link into its page.
    fn link(&self) -> String {
        format!(
            "<a class=\"{LINK_CLASSES}\" href=\"/run/{name}\">{name}</a>",
            name = escape(&self.name)
        )
    }

    /// The run cell of the runs table: the link with the age beneath it.
    fn run_cell(&self) -> String {
        format!(
            "{}<div class=\"text-xs {MUTED_CLASSES} mt-0.5\">{} ago</div>",
            self.link(),
            age(self.started())
        )
    }

    /// The state of the run as a pill.
    fn state(&self) -> String {
        if self.live {
            pill(
                LIVE_PILL,
                true,
                if self.last_call() {
                    "last call"
                } else {
                    "live"
                },
            )
        } else if self.solved() {
            pill(SOLVED_PILL, false, "solved")
        } else if self.score.is_some() {
            pill(UNSOLVED_PILL, false, "unsolved")
        } else {
            pill(FAILED_PILL, false, "no score")
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
            None => epoch_now().saturating_sub(self.started()),
        }
        .min(self.limit())
    }

    /// How far into its time budget a live run is, as a meter.
    fn elapsed_meter(&self) -> String {
        let elapsed = self.elapsed();
        meter(
            elapsed,
            self.limit(),
            time_fill(elapsed, self.limit()),
            &format!("{elapsed}/{}s", self.limit()),
            ELAPSED_LABEL_WIDTH,
        )
    }

    /// The time cell of the runs table: the same meter against the budget for
    /// every run.
    fn time_cell(&self) -> String {
        if self.live {
            return self.elapsed_meter();
        }

        let wall = self.wall.unwrap_or(0);
        meter(
            wall,
            self.limit(),
            time_fill(wall, self.limit()),
            &format!("{wall}/{}s", self.limit()),
            ELAPSED_LABEL_WIDTH,
        )
    }

    /// The pushes cell of the runs table: what was scored so far while live,
    /// the count of record once scored, nothing for a run that left neither.
    fn pushes_cell(&self) -> String {
        if self.live || !self.attempts.is_empty() {
            return self.attempts.len().to_string();
        }

        match self.score {
            Some(_) => self.attempts().to_string(),
            None => String::new(),
        }
    }

    /// The requests a model answered without ever reporting their usage.
    ///
    /// Blank when there were none. It is what tells a run that spent its
    /// budget on streams which never finished apart from one where the model
    /// simply did not solve the task, since both score zero.
    fn truncated_cell(&self) -> String {
        match self.metric("truncated_requests") {
            0 => String::new(),
            cut => cut.to_string(),
        }
    }

    /// The points cell of the runs table: the best solving push so far while
    /// live, the points of record once scored.
    fn points_cell(&self) -> String {
        if self.live {
            return points_meter(
                best_push(&self.attempts).map_or(0, |push| number(push, "points")),
            );
        }

        match self.score {
            Some(_) => points_meter(self.points()),
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
            escape(self.game()),
            escape(self.model()),
            self.agent(),
            self.time_cell(),
            self.pushes_cell(),
            self.truncated_cell(),
            self.points_cell(),
            self.stop_form(),
        ]
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
                run.harness() == start.agent
                    && run.model() == start.model
                    && run.game() == start.game
                    && run.started() + 1 >= start.started
            })
            .count() as u64;

        for _ in appeared..start.parallel {
            rows.push(vec![
                format!(
                    "<span class=\"{MUTED_CLASSES}\">pending</span><div class=\"text-xs {MUTED_CLASSES} mt-0.5\">asked {} ago</div>",
                    age(start.started)
                ),
                pill(STARTING_PILL, true, "starting"),
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

    let harnesses = registry
        .harnesses
        .iter()
        .map(|harness| harness.name.as_str())
        .collect::<Vec<_>>();
    let models = registry
        .models
        .iter()
        .map(|model| model.name.as_str())
        .collect::<Vec<_>>();
    let games = games()?;
    let games = games.iter().map(String::as_str).collect::<Vec<_>>();

    let mut levels = vec![""];
    levels.extend(registry::THINKING_LEVELS);

    let limit = selection
        .limit
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(docker::Agent::DEFAULT_LIMIT_SECONDS);
    let parallel = selection
        .parallel
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(docker::Agent::DEFAULT_PARALLEL_RUNS);

    Ok(format!(
        "<p class=\"{FIRST_TITLE_CLASSES}\">new run</p>\
         <form method=\"post\" action=\"/start\" class=\"{CARD_CLASSES} p-4 flex flex-wrap items-end gap-4\">\
         {}{}{}{}\
         <label class=\"w-24\"><span class=\"{LABEL_CLASSES}\">seconds</span>\
         <input class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" type=\"number\" name=\"limit\" value=\"{limit}\" min=\"1\"></label>\
         <label class=\"w-20\"><span class=\"{LABEL_CLASSES}\">parallel</span>\
         <input class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" type=\"number\" name=\"parallel\" value=\"{parallel}\" min=\"1\"></label>\
         <label class=\"{CONTROL_HEIGHT} flex items-center gap-2\">\
         <input type=\"checkbox\" name=\"force\" class=\"h-4 w-4 rounded accent-indigo-500\">\
         <span class=\"{NOTE_CLASSES}\">rebuild images</span></label>\
         <button class=\"{BUTTON_CLASSES} {CONTROL_HEIGHT}\">start</button>\
         </form>",
        select(
            "agent",
            &harnesses,
            selection.agent.as_deref().unwrap_or("")
        ),
        select("model", &models, selection.model.as_deref().unwrap_or("")),
        select(
            "game",
            &games,
            selection.game.as_deref().unwrap_or(DEFAULT_GAME)
        ),
        select(
            "thinking",
            &levels,
            selection.thinking.as_deref().unwrap_or(DEFAULT_THINKING)
        ),
    ))
}

/// A labeled dropdown named `name` offering `options`, with `selected` marked.
///
/// The dropdowns grow to fill the form row, which puts the right edge of the
/// form on the edge the tables end on.
fn select(name: &str, options: &[&str], selected: &str) -> String {
    let mut rendered = format!(
        "<label class=\"grow basis-44\"><span class=\"{LABEL_CLASSES}\">{name}</span>\
         <select class=\"{FIELD_CLASSES} {CONTROL_HEIGHT}\" name=\"{name}\">"
    );
    for option in options {
        let marked = if *option == selected { " selected" } else { "" };
        rendered.push_str(&format!("<option{marked}>{}</option>", escape(option)));
    }
    rendered.push_str("</select></label>");
    rendered
}

/// One run: its state and figures first, the pushes, the console, then the rest.
pub(crate) fn run_page(name: &str, notice: &Notice) -> std::io::Result<String> {
    let directory = run_directory(name)?;

    let metadata = read_json(&directory.join(docker::METADATA_FILE)).unwrap_or_default();
    let score = read_json(&directory.join(docker::SCORE_FILE));
    let live = live_runs()
        .iter()
        .any(|running| running == &docker::sandbox_container(name));
    let entry = RunEntry {
        name: name.to_string(),
        live,
        wall: wall_seconds(&directory, number(&metadata, "started_seconds")),
        monitor: read_json(&directory.join(docker::MONITOR_FILE)),
        attempts: attempts_of(&directory, name, live),
        metadata,
        score,
    };

    let mut body = format!(
        "<div data-refresh=\"run\"><div class=\"flex items-center gap-3\">\
         <span class=\"text-lg font-semibold text-neutral-100 {MONO_CLASSES}\">{}</span>{}{}</div>",
        escape(name),
        entry.state(),
        entry.stop_form()
    );
    body.push_str(&notice.render());

    body.push_str(&format!(
        "<p class=\"{NOTE_CLASSES} mt-1.5\">{} \u{00b7} {} \u{00b7} {} \u{00b7} started {} ago</p>",
        escape(entry.game()),
        escape(entry.model()),
        entry.agent(),
        age(entry.started()),
    ));

    let solved_at = match (entry.live, best_push(&entry.attempts)) {
        (true, Some(push)) => format!("{}s", number(push, "seconds")),
        (false, _) if entry.solved() => format!("{}s", entry.seconds()),
        _ => "not solved".to_string(),
    };
    let tiles = [
        ("points", entry.points_cell()),
        ("pushes", entry.pushes_cell()),
        ("solved at", solved_at),
        ("time", entry.time_cell()),
        ("requests", entry.metric("requests").to_string()),
        ("output tokens", entry.metric("output_tokens").to_string()),
    ];
    body.push_str("<div class=\"mt-6 grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-3\">");
    for (label, value) in tiles {
        body.push_str(&format!(
            "<div class=\"{TILE_CLASSES}\"><p class=\"{TILE_LABEL_CLASSES}\">{label}</p><div class=\"{TILE_VALUE_CLASSES}\">{value}</div></div>"
        ));
    }
    body.push_str("</div>");

    let pushes = attempt_rows(&entry.attempts);
    if !pushes.is_empty() {
        body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">pushes</p>"));
        body.push_str(&table(
            &["#SECONDS", "STATE", "POINTS", "*REASON"],
            pushes,
            None,
        ));
    }

    if let Some(metrics) = entry
        .score
        .as_ref()
        .and_then(|report| report.get("metrics"))
    {
        body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">metrics</p>"));
        body.push_str(&object_table(metrics));
    }

    if let Ok(console) = std::fs::read(directory.join(docker::AGENT_LOG)) {
        let tail = console.len().saturating_sub(CONSOLE_TAIL_BYTES);
        body.push_str(&format!(
            "<p class=\"{TITLE_CLASSES}\">console <span class=\"{NOTE_CLASSES} font-normal\">the last {} bytes</span></p><pre class=\"{CONSOLE_CLASSES}\">{}</pre>",
            console.len() - tail,
            escape(&strip_ansi(&String::from_utf8_lossy(&console[tail..])))
        ));
    }

    let files = RUN_FILES
        .iter()
        .filter(|file| directory.join(file).exists())
        .map(|file| {
            format!(
                "<a class=\"{LINK_CLASSES} mr-4\" href=\"/run/{}/{file}\">{file}</a>",
                escape(name)
            )
        })
        .collect::<String>();
    body.push_str(&format!(
        "<p class=\"{TITLE_CLASSES}\">files</p><p>{files}</p>"
    ));

    body.push_str(&format!(
        "<details class=\"mt-8\"><summary class=\"{SUMMARY_CLASSES}\">parameters</summary><div class=\"mt-3\">{}</div></details>",
        object_table(&entry.metadata)
    ));
    body.push_str("</div>");

    Ok(page(name, &body))
}

/// The best of every played pairing, grouped over the finished runs.
pub(crate) fn scoreboard_page() -> std::io::Result<String> {
    struct Standing {
        runs: u64,
        solved: u64,
        points: u64,
        seconds: u64,
    }

    let runs = collect_runs()?;
    let mut standings: Vec<(String, String, String, Standing)> = Vec::new();

    for run in runs.iter().filter(|run| run.score.is_some()) {
        let key = (run.game().to_string(), run.model().to_string(), run.agent());

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
                        solved: 0,
                        points: 0,
                        seconds: 0,
                    },
                ));
                &mut standings.last_mut().expect("just pushed").3
            }
        };

        standing.runs += 1;
        standing.solved += u64::from(run.solved());
        if run.comparable() && run.points() > standing.points {
            standing.points = run.points();
            standing.seconds = run.seconds();
        }
    }

    standings.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then(right.3.points.cmp(&left.3.points))
    });

    let rows = standings
        .iter()
        .map(|(game, model, agent, standing)| {
            vec![
                escape(game),
                escape(model),
                agent.clone(),
                standing.runs.to_string(),
                standing.solved.to_string(),
                points_meter(standing.points),
                standing.seconds.to_string(),
            ]
        })
        .collect();

    let body = format!(
        "<p class=\"{FIRST_TITLE_CLASSES}\">scoreboard <span class=\"{NOTE_CLASSES} font-normal\">the best run of every pairing</span></p>{}",
        table(
            &[
                "GAME", "MODEL", "HARNESS", "#RUNS", "#SOLVED", "*BEST", "#SECONDS",
            ],
            rows,
            Some("nothing scored yet"),
        )
    );

    Ok(page("scoreboard", &body))
}

/// Every game: its task, its record and its standings.
pub(crate) fn games_page() -> std::io::Result<String> {
    let runs = collect_runs()?;
    let mut body = String::new();

    for (index, game) in games()?.into_iter().enumerate() {
        let played: Vec<&RunEntry> = runs.iter().filter(|run| run.game() == game).collect();
        let solved = played.iter().filter(|run| run.solved()).count();
        let mut standing: Vec<&&RunEntry> = played
            .iter()
            .filter(|run| run.solved() && run.comparable())
            .collect();
        standing.sort_by_key(|run| std::cmp::Reverse(run.points()));

        let record = match standing.first() {
            Some(best) => format!(
                "{} runs, {solved} solved, the record is {} by {} on {}",
                played.len(),
                best.points(),
                escape(best.model()),
                best.agent()
            ),
            None if played.is_empty() => "not played yet".to_string(),
            None => format!("{} runs, none solving under the ceiling", played.len()),
        };

        let title_classes = if index == 0 {
            FIRST_TITLE_CLASSES
        } else {
            TITLE_CLASSES
        };
        body.push_str(&format!(
            "<p class=\"{title_classes}\">{} <span class=\"{NOTE_CLASSES} font-normal\">{record}</span></p>",
            escape(&game)
        ));

        let task = std::fs::read_to_string(
            std::path::Path::new(GAMES_DIRECTORY)
                .join(&game)
                .join(TASK_FILE),
        )
        .unwrap_or_default();

        let standings: Vec<Vec<String>> = standing
            .iter()
            .take(STANDINGS_LIMIT)
            .map(|run| {
                vec![
                    escape(run.model()),
                    run.agent(),
                    points_meter(run.points()),
                    run.seconds().to_string(),
                    run.link(),
                ]
            })
            .collect();

        body.push_str(&format!(
            "<div class=\"{CARD_CLASSES} overflow-hidden\"><div class=\"px-4 pb-4\">{}</div>{}</div>",
            crate::markdown::render(&task),
            table(
                &["MODEL", "HARNESS", "*POINTS", "#SECONDS", "RUN"],
                standings,
                None,
            )
        ));
    }

    if let Ok(instructions) =
        std::fs::read_to_string(std::path::Path::new(GAMES_DIRECTORY).join(INSTRUCTIONS_FILE))
    {
        body.push_str(&format!(
            "<details class=\"mt-8\"><summary class=\"{SUMMARY_CLASSES}\">the instructions shared by every game</summary><div class=\"{CARD_CLASSES} mt-3 px-4 pb-4\">{}</div></details>",
            crate::markdown::render(&instructions)
        ));
    }

    Ok(page("games", &body))
}

/// The registry, the credentials and the docker images runs are built from.
pub(crate) fn setup_page() -> std::io::Result<String> {
    let registry = registry::load()?;
    let runs = collect_runs()?;

    let mut backend_rows = Vec::new();
    let mut limit_rows = Vec::new();
    let mut raw_limits = String::new();
    let mut reported: Vec<String> = Vec::new();
    for backend in &registry.backends {
        let state = if std::env::var(&backend.key).is_ok() {
            pill(SOLVED_PILL, false, "set")
        } else {
            pill(FAILED_PILL, false, "missing")
        };

        // The proxy metrics name every host a run requested, which is what
        // attributes the run to the backend that served it.
        let fed: Vec<&RunEntry> = runs
            .iter()
            .filter(|run| run.hosts().contains(&backend.host.as_str()))
            .collect();

        let mut row = vec![
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
            fed.len().to_string(),
        ];
        for metric in [
            "requests",
            "input_tokens",
            "output_tokens",
            "cache_read_tokens",
            "cache_write_tokens",
        ] {
            row.push(
                fed.iter()
                    .map(|run| run.metric(metric))
                    .sum::<u64>()
                    .to_string(),
            );
        }
        row.push(money(
            fed.iter().map(|run| run.metric_float("gateway_cost")).sum(),
        ));
        backend_rows.push(row);

        // The runs are newest first, so the first captured set is the
        // freshest view of the account.
        if let Some(run) = fed
            .iter()
            .find(|run| !run.metric_text("ratelimits").is_empty())
        {
            let captured = run.metric_text("ratelimits");
            limit_rows.extend(limit_rows_of(&backend.name, captured));
            reported.push(format!(
                "{} {} ago",
                escape(&backend.name),
                age(run.started())
            ));
            raw_limits.push_str(&format!(
                "<p class=\"mt-2\"><span class=\"{NOTE_CLASSES}\">{}, {} ago:</span> <span class=\"{MONO_CLASSES} text-xs text-neutral-300 break-all\">{}</span></p>",
                escape(&backend.name),
                age(run.started()),
                escape(captured)
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
        "<p class=\"{FIRST_TITLE_CLASSES}\">backends <span class=\"{NOTE_CLASSES} font-normal\">with the key of each and the usage recorded over every run on disk</span></p>"
    );
    body.push_str(&table(
        &[
            "BACKEND",
            "SERVICE",
            "HOST",
            "KEY",
            "*STATE",
            "#RUNS",
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
        "<p class=\"{TITLE_CLASSES}\">limits <span class=\"{NOTE_CLASSES} font-normal\">as the newest answer of each backend reported them{}</span></p>",
        if reported.is_empty() {
            String::new()
        } else {
            format!(", {}", reported.join(", "))
        }
    ));
    body.push_str(&table(
        &["BACKEND", "WINDOW", "*USED", "LEFT", "STATUS", "RESETS"],
        limit_rows,
        Some(NO_LIMITS_NOTE),
    ));
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

    if let Ok(listing) = process::run_and_assume_success(
        "docker",
        &[
            "image",
            "ls",
            "--format",
            "{{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}",
        ],
    ) {
        let rows = listing
            .lines()
            .filter(|line| line.starts_with("ava/"))
            .map(|line| line.split('\t').map(escape).collect())
            .collect();
        body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">images</p>"));
        body.push_str(&table(&["IMAGE", "TAG", "SIZE", "CREATED"], rows, None));
    }

    Ok(page("setup", &body))
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
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
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

/// The names of the containers running right now.
fn live_runs() -> Vec<String> {
    process::run_and_assume_success("docker", &["ps", "--format", "{{.Names}}"])
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// Every run on disk, newest first, marked live while its sandbox is up.
///
/// A run directory that is not there holds no runs, which is what a fresh
/// checkout looks like until the first run creates it.
fn collect_runs() -> std::io::Result<Vec<RunEntry>> {
    let running = live_runs();

    let played = match std::fs::read_dir(docker::RUN_DIRECTORY) {
        Ok(played) => played,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(at_path(docker::RUN_DIRECTORY, error)),
    };

    let mut runs = Vec::new();
    for entry in played.filter_map(Result::ok) {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Some(metadata) = read_json(&entry.path().join(docker::METADATA_FILE)) else {
            continue;
        };

        let sandbox = docker::sandbox_container(&name);
        let live = running.iter().any(|container| container == &sandbox);
        runs.push(RunEntry {
            live,
            score: read_json(&entry.path().join(docker::SCORE_FILE)),
            wall: wall_seconds(&entry.path(), number(&metadata, "started_seconds")),
            monitor: read_json(&entry.path().join(docker::MONITOR_FILE)),
            attempts: attempts_of(&entry.path(), &name, live),
            metadata,
            name,
        });
    }

    runs.sort_by_key(|run| std::cmp::Reverse(number(&run.metadata, "started_seconds")));

    Ok(runs)
}

/// The scored pushes of a run: the collected attempts log once the run is
/// over, the output of the scoring container while it is live, since that
/// output is the very log collected at the end.
fn attempts_of(directory: &std::path::Path, name: &str, live: bool) -> Vec<serde_json::Value> {
    let contents = if live {
        std::process::Command::new("docker")
            .args(["logs", &docker::scorer_container(name)])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
            .unwrap_or_default()
    } else {
        std::fs::read_to_string(directory.join(docker::SCORE_LOG)).unwrap_or_default()
    };

    contents
        .lines()
        .filter(|line| line.starts_with('{'))
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// The best solving push among `attempts`, the earliest one breaking ties.
fn best_push(attempts: &[serde_json::Value]) -> Option<&serde_json::Value> {
    attempts
        .iter()
        .filter(|push| {
            push.get("solved")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
        })
        .max_by_key(|push| {
            (
                number(push, "points"),
                std::cmp::Reverse(number(push, "seconds")),
            )
        })
}

/// The scored pushes as table rows.
fn attempt_rows(attempts: &[serde_json::Value]) -> Vec<Vec<String>> {
    attempts
        .iter()
        .map(|push| {
            let solved = push
                .get("solved")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            vec![
                number(push, "seconds").to_string(),
                if solved {
                    pill(SOLVED_PILL, false, "solved")
                } else {
                    pill(UNSOLVED_PILL, false, "unsolved")
                },
                points_meter(number(push, "points")),
                escape(text(push, "reason")),
            ]
        })
        .collect()
}

/// A flat JSON object as a two column table in a card.
fn object_table(value: &serde_json::Value) -> String {
    let Some(object) = value.as_object() else {
        return String::new();
    };

    let mut html = format!(
        "<div class=\"{CARD_CLASSES} overflow-hidden\"><table class=\"{TABLE_CLASSES}\"><tbody>"
    );
    for (index, (key, value)) in object.iter().enumerate() {
        let border = if index == 0 { "border-t-0" } else { "" };
        html.push_str(&format!(
            "<tr class=\"{ROW_CLASSES}\"><td class=\"{PACKED_COLUMN_CLASSES} {CELL_CLASSES} {border} text-neutral-400\">{}</td><td class=\"{SLACK_COLUMN_CLASSES} {CELL_CLASSES} {border} {MONO_CLASSES} text-neutral-200\">{}</td></tr>",
            escape(key),
            escape(&plain(value))
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

fn text<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(|field| field.as_str())
        .unwrap_or("")
}

fn number(value: &serde_json::Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

fn pointer(value: Option<&serde_json::Value>, path: &str) -> u64 {
    value
        .and_then(|value| value.pointer(path))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

/// The wall clock seconds a finished run took, from its start to the moment
/// its report was written, since nothing records the end explicitly.
fn wall_seconds(directory: &std::path::Path, started: u64) -> Option<u64> {
    let finished = [docker::SCORE_FILE, docker::AGENT_LOG]
        .iter()
        .find_map(|file| {
            std::fs::metadata(directory.join(file))
                .ok()?
                .modified()
                .ok()
        })?;

    let finished = finished
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    Some(finished.saturating_sub(started))
}

fn epoch_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

/// How long ago the epoch second `started` was, in the largest two units.
fn age(started: u64) -> String {
    let elapsed = epoch_now().saturating_sub(started);

    match elapsed {
        seconds if seconds < 60 => format!("{seconds}s"),
        seconds if seconds < 3600 => format!("{}m", seconds / 60),
        seconds if seconds < 86400 => format!("{}h {}m", seconds / 3600, seconds % 3600 / 60),
        seconds => format!("{}d {}h", seconds / 86400, seconds % 86400 / 3600),
    }
}

/// The limit rows of one backend, read out of the headers `captured` from its
/// newest answer.
fn limit_rows_of(backend: &str, captured: &str) -> Vec<Vec<String>> {
    let headers: std::collections::BTreeMap<&str, &str> = captured
        .split_whitespace()
        .filter_map(|pair| pair.split_once('='))
        .collect();
    let header = |name: String| headers.get(name.as_str()).copied();
    let backend_cell = format!("<span class=\"{MONO_CLASSES}\">{}</span>", escape(backend));
    let mut rows = Vec::new();

    for (window, label) in ANTHROPIC_WINDOWS {
        let Some(utilization) = header(format!("{ANTHROPIC_LIMIT_PREFIX}{window}-utilization"))
        else {
            continue;
        };
        let used = (utilization.parse::<f64>().unwrap_or(0.0) * 100.0).round() as u64;
        rows.push(vec![
            backend_cell.clone(),
            label.to_string(),
            meter(
                used,
                100,
                USAGE_FILL,
                &format!("{used}%"),
                USAGE_LABEL_WIDTH,
            ),
            format!("{}%", 100u64.saturating_sub(used)),
            status_pill(header(format!("{ANTHROPIC_LIMIT_PREFIX}{window}-status"))),
            header(format!("{ANTHROPIC_LIMIT_PREFIX}{window}-reset"))
                .and_then(|epoch| epoch.parse().ok())
                .map(utc_date)
                .unwrap_or_default(),
        ]);
    }

    if let Some(status) = header(format!("{ANTHROPIC_LIMIT_PREFIX}overage-status")) {
        let reason = header(format!("{ANTHROPIC_LIMIT_PREFIX}overage-disabled-reason"))
            .map(|reason| format!(" <span class=\"{MUTED_CLASSES}\">{}</span>", escape(reason)))
            .unwrap_or_default();
        rows.push(vec![
            backend_cell.clone(),
            "overage".to_string(),
            String::new(),
            String::new(),
            format!("{}{reason}", status_pill(Some(status))),
            String::new(),
        ]);
    }

    if let (Some(spend), Some(budget)) = (
        header(GATEWAY_SPEND_HEADER.to_string()).and_then(|value| value.parse::<f64>().ok()),
        header(GATEWAY_BUDGET_HEADER.to_string()).and_then(|value| value.parse::<f64>().ok()),
    ) {
        rows.push(vec![
            backend_cell.clone(),
            "budget".to_string(),
            meter(
                (spend * 100.0) as u64,
                (budget * 100.0) as u64,
                USAGE_FILL,
                &money(spend),
                USAGE_LABEL_WIDTH,
            ),
            format!("{} of {}", money((budget - spend).max(0.0)), money(budget)),
            String::new(),
            String::new(),
        ]);
    }

    for window in GATEWAY_WINDOWS {
        let (Some(limit), Some(remaining)) = (
            header(format!("{GATEWAY_LIMIT_PREFIX}limit-{window}"))
                .and_then(|value| value.parse::<u64>().ok()),
            header(format!("{GATEWAY_LIMIT_PREFIX}remaining-{window}"))
                .and_then(|value| value.parse::<u64>().ok()),
        ) else {
            continue;
        };
        let used = limit.saturating_sub(remaining);
        rows.push(vec![
            backend_cell.clone(),
            window.to_string(),
            meter(
                used,
                limit,
                USAGE_FILL,
                &used.to_string(),
                USAGE_LABEL_WIDTH,
            ),
            format!("{remaining} of {limit}"),
            String::new(),
            header(format!("{GATEWAY_LIMIT_PREFIX}reset-{window}"))
                .map(escape)
                .unwrap_or_default(),
        ]);
    }

    rows
}

/// A limit status as a pill: allowed is fine, a warning is amber, a
/// rejection is red, anything else is neutral.
fn status_pill(status: Option<&str>) -> String {
    let Some(status) = status else {
        return String::new();
    };

    let tint = match status {
        "allowed" => SOLVED_PILL,
        "allowed_warning" => STARTING_PILL,
        "rejected" => FAILED_PILL,
        _ => NEUTRAL_PILL,
    };

    pill(tint, false, &escape(status))
}

/// A dollar amount with cents.
fn money(amount: f64) -> String {
    format!("${amount:.2}")
}

/// The epoch second `epoch` as a UTC date and time, to the minute.
fn utc_date(epoch: u64) -> String {
    let days = (epoch / SECONDS_PER_DAY) as i64;
    let seconds = epoch % SECONDS_PER_DAY;

    // Civil date from days since the epoch, after Howard Hinnant.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        seconds / SECONDS_PER_HOUR,
        seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE
    )
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

/// A state pill, with a pulsing dot for a state still changing.
fn pill(tint: &str, pulsing: bool, label: &str) -> String {
    let dot = if pulsing {
        "<span class=\"h-1.5 w-1.5 rounded-full bg-current animate-pulse\"></span>"
    } else {
        ""
    };

    format!("<span class=\"{PILL_CLASSES} {tint}\">{dot}{label}</span>")
}

/// A column title, carrying its tooltip when the header named one.
fn heading(title: &str, tooltip: &str) -> String {
    if tooltip.is_empty() {
        return title.to_string();
    }

    format!(
        "<span class=\"{TOOLTIP_CLASSES}\" title=\"{}\">{title}</span>",
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
        POINT_CEILING,
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
    let share = 100 / slack.len();
    let column = |index: usize| {
        if slack.contains(&index) {
            (SLACK_COLUMN_CLASSES, format!(" style=\"width:{share}%\""))
        } else {
            (PACKED_COLUMN_CLASSES, String::new())
        }
    };

    let mut html = format!(
        "<div class=\"{CARD_CLASSES} overflow-x-auto\"><table class=\"{TABLE_CLASSES}\"><thead><tr>"
    );
    for (index, (header, numeric)) in headers.iter().zip(&numeric).enumerate() {
        let align = if *numeric { "text-right" } else { "text-left" };
        let (classes, style) = column(index);
        let (title, tooltip) = header.split_once(TOOLTIP_SEPARATOR).unwrap_or((header, ""));
        html.push_str(&format!(
            "<th class=\"{classes} {HEADER_CLASSES} {align}\"{style}>{}</th>",
            heading(
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
            let (classes, style) = column(index);
            html.push_str(&format!(
                "<td class=\"{classes} {CELL_CLASSES} {align}\"{style}>{cell}</td>"
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
