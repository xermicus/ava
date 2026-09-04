//! The pages of the web interface, rendered from the run and tournament
//! records on disk, the registry and the games folder.

use ava_game::scoring::Scoring;
use ava_run::{docker, process, registry, runs, tournament, usage};

const GAMES_DIRECTORY: &str = "games";
const TASK_DIRECTORY: &str = "task";
const TASK_FILE: &str = "task.md";
const INSTRUCTIONS_FILE: &str = "README.md";

/// How many passing runs a game lists on its standings.
const STANDINGS_LIMIT: usize = 3;

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
const RUN_HEADERS: [&str; 10] = [
    "RUN|the run directory under runs/, how long ago it started, and the tournament seat it plays",
    "STATE|live or the last call while the run goes, whether a push passed the verifier once it is over",
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
const PASSED_PILL: &str = "bg-emerald-500/10 text-emerald-400";
const FAILED_PILL: &str = "bg-orange-500/10 text-orange-400";
const BROKEN_PILL: &str = "bg-red-500/10 text-red-400";
const STARTING_PILL: &str = "bg-amber-500/10 text-amber-400";
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
    /// Whether an analyst is up for the run.
    analyzing: bool,
    /// The newest heartbeat of the run loop, for a live run.
    monitor: Option<serde_json::Value>,
    /// The pushes graded so far: the record once the run is over, the output
    /// of the scoring container while it is live.
    attempts: Vec<ava_wire::Attempt>,
    /// The entry of record, once the run is over and kept one.
    record: Option<runs::Entry>,
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
            ava_game::find(&run.game)
                .and_then(|game| runs::entry_of_record(game, directory).ok().flatten())
        };

        Self {
            live,
            analyzing: running.contains(&docker::analyst_container(&name)),
            monitor: read_json(&directory.join(docker::MONITOR_FILE)),
            attempts: attempts_of(directory, &name, live, &run),
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
        format!(
            "<a class=\"{LINK_CLASSES}\" href=\"/run/{name}\">{name}</a>",
            name = escape(&self.name)
        )
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
                placement_label(placement)
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

        if self.analyzing {
            pill(STARTING_PILL, true, "analyzing")
        } else if self.passed() {
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
            .run
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
            escape(&self.run.game),
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
    let games = startable_games()?;
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
         <div class=\"hidden peer-checked:contents\">{}</div>\
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
         {}<button class=\"{BUTTON_CLASSES} {CONTROL_HEIGHT}\">analyze</button></form>",
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
    ))
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
         <span class=\"text-lg font-semibold text-neutral-100 {MONO_CLASSES}\">{}</span>{}{}</div>",
        escape(name),
        entry.state(),
        entry.stop_form()
    );
    body.push_str(&notice.render());

    body.push_str(&format!(
        "<p class=\"{NOTE_CLASSES} mt-1.5\">{} {} \u{00b7} {} \u{00b7} {} {} \u{00b7} started {} ago</p>",
        escape(&entry.run.game),
        version_label(&entry.run.game_version),
        escape(&entry.run.model),
        entry.agent(),
        version_label(&entry.run.harness_version),
        usage::age(entry.run.started_seconds),
    ));
    if let Some(placement) = &entry.placement {
        body.push_str(&format!(
            "<p class=\"{NOTE_CLASSES} mt-1\">{}</p>",
            placement_label(placement)
        ));
    }
    if let Some(challenge) = &entry.run.challenge {
        body.push_str(&format!(
            "<p class=\"{NOTE_CLASSES} mt-1\">attacking the entry <a class=\"{LINK_CLASSES}\" href=\"/run/{run}\">{run}</a> kept at {}s</p>",
            challenge.attempt,
            run = escape(&challenge.run)
        ));
    }

    if !entry.live {
        let report = directory.join(docker::ANALYSIS_FILE);
        let analysis = read_json(&report);
        let failed = analysis
            .as_ref()
            .is_some_and(|analysis| analysis.get(docker::ANALYSIS_ERROR).is_some());
        let analyzed = match runs::modified_seconds(&report) {
            Some(written) if failed => format!("failed {} ago", usage::age(written)),
            Some(written) => format!("analyzed {} ago", usage::age(written)),
            None => "not analyzed yet".to_string(),
        };
        body.push_str(&format!(
            "<p class=\"{TITLE_CLASSES}\">analysis <span class=\"{NOTE_CLASSES} font-normal\">{analyzed}</span></p>"
        ));
        if let Some(analysis) = analysis {
            if failed {
                body.push_str(&format!(
                    "<p class=\"mb-4 rounded-md border border-red-500/30 bg-red-500/10 px-3 py-2 text-red-300\">{}</p>",
                    escape(text(&analysis, docker::ANALYSIS_ERROR))
                ));
            } else {
                body.push_str(&format!(
                    "<div class=\"{CARD_CLASSES} px-4 py-3 mb-4\"><p class=\"max-w-prose leading-relaxed text-neutral-200\">{}</p>\
                     <details class=\"mt-3\"><summary class=\"{SUMMARY_CLASSES}\">the full analysis</summary><div class=\"mt-2\">{}</div></details></div>",
                    escape(text(&analysis, "analysis_summary")),
                    crate::markdown::render(text(&analysis, "analysis"))
                ));
            }
        }
        if !entry.analyzing {
            body.push_str(&analysis_panel(name)?);
        }
    }

    let entry_at = match &entry.record {
        Some(record) => format!("{}s", record.seconds),
        None if entry.live => String::new(),
        None => "no entry".to_string(),
    };
    let metric = |value: fn(&ava_wire::Metrics) -> u64| {
        entry
            .run
            .metrics
            .as_ref()
            .map(|metrics| value(metrics).to_string())
            .unwrap_or_default()
    };
    let tiles = [
        ("points", entry.points_cell()),
        ("pushes", entry.attempts.len().to_string()),
        ("entry at", entry_at),
        ("time", entry.time_cell()),
        ("requests", metric(|metrics| metrics.requests)),
        ("output tokens", metric(|metrics| metrics.output_tokens)),
    ];
    body.push_str("<div class=\"mt-6 grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-3\">");
    for (label, value) in tiles {
        body.push_str(&format!(
            "<div class=\"{TILE_CLASSES}\"><p class=\"{TILE_LABEL_CLASSES}\">{label}</p><div class=\"{TILE_VALUE_CLASSES}\">{value}</div></div>"
        ));
    }
    body.push_str("</div>");

    if let Some(game) = ava_game::find(&entry.run.game)
        && !entry.live
    {
        let kept = runs::entries(game, &directory)?;
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
                            escape(game.entry()),
                            escape(game.entry())
                        ),
                    ]
                })
                .collect();
            body.push_str(&format!(
                "<p class=\"{TITLE_CLASSES}\">entries <span class=\"{NOTE_CLASSES} font-normal\">what the passing pushes left, ranked as the game ranks them today</span></p>"
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

    if let Some(metrics) = &entry.run.metrics {
        body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">metrics</p>"));
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
        "<details class=\"mt-8\"><summary class=\"{SUMMARY_CLASSES}\">parameters</summary><div class=\"mt-3\">{}</div></details>",
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
        let key = (run.run.game.clone(), run.run.model.clone(), run.agent());

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
                escape(game),
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
        "<p class=\"{FIRST_TITLE_CLASSES}\">scoreboard <span class=\"{NOTE_CLASSES} font-normal\">the best entry of every pairing, ranked as the games rank today</span></p>{}",
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

/// Every game: its task, its record and its standings.
pub(crate) fn games_page() -> std::io::Result<String> {
    let runs = collect_runs()?;
    let mut body = String::new();

    for (index, game) in games()?.into_iter().enumerate() {
        let played: Vec<&RunEntry> = runs.iter().filter(|run| run.run.game == game).collect();
        let passed = played.iter().filter(|run| run.passed()).count();
        let mut standing: Vec<(&RunEntry, &runs::Entry)> = played
            .iter()
            .filter_map(|run| run.record.as_ref().map(|record| (*run, record)))
            .collect();
        standing.sort_by_key(|(_, record)| std::cmp::Reverse((record.points, record.seconds)));

        let playout = ava_game::find(&game)
            .filter(|game| game.playout() == ava_game::Playout::Automated)
            .map(|_| ", played in tournaments")
            .unwrap_or_default();
        let record = match standing.first() {
            Some((best, record)) => match record.points {
                Some(points) => format!(
                    "{} runs, {passed} passed, the record is {points} by {} on {}{playout}",
                    played.len(),
                    escape(&best.run.model),
                    best.agent()
                ),
                None => format!("{} runs, {passed} passed{playout}", played.len()),
            },
            None if played.is_empty() => format!("not played yet{playout}"),
            None => format!("{} runs, none passing{playout}", played.len()),
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
                .join(TASK_DIRECTORY)
                .join(TASK_FILE),
        )
        .unwrap_or_default();

        let standings: Vec<Vec<String>> = standing
            .iter()
            .take(STANDINGS_LIMIT)
            .map(|(run, record)| {
                vec![
                    escape(&run.run.model),
                    run.agent(),
                    record.points.map(points_meter).unwrap_or_default(),
                    record.seconds.to_string(),
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

/// The tournaments: the form opening one, and every tournament on disk.
pub(crate) fn tournaments_page(notice: &Notice, selection: &Selection) -> std::io::Result<String> {
    let games: Vec<&str> = ava_game::GAMES
        .iter()
        .map(|game| game.name())
        .filter(|game| ava_game::attacked_by(game).is_none())
        .collect();
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
         <div class=\"hidden peer-checked:contents\">{}</div>\
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
            "the combats every fight of an automated playout plays, each best of three rounds",
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
    let playout = ava_game::find(&record.game).map(|game| game.playout());
    let compared = playout == Some(ava_game::Playout::Single);
    let ordered = matches!(playout, Some(ava_game::Playout::Played { .. }));

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
            .map(|analyst| format!(" \u{00b7} analyzed by {}", escape(&analyst.label())))
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
        "<div data-refresh=\"lobby\"><p class=\"{TITLE_CLASSES}\">lobby <span class=\"{NOTE_CLASSES} font-normal\">{} seats{}</span></p>{}</div>",
        record.seats.len(),
        if record.played() {
            ", fixed by the rounds played"
        } else {
            ""
        },
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
        let matches = tournament::matches(&record)?.len();
        let rows = standings
            .iter()
            .map(|standing| {
                vec![
                    escape(&standing.agent),
                    standing.seats.to_string(),
                    standing.matches.to_string(),
                    standing.tally.won.to_string(),
                    standing.tally.drawn.to_string(),
                    standing.tally.lost.to_string(),
                    rating_label(standing.elo),
                    rating_label(standing.bradley_terry),
                ]
            })
            .collect();
        body.push_str(&format!(
            "<p class=\"{TITLE_CLASSES}\">standings <span class=\"{NOTE_CLASSES} font-normal\">derived from the {matches} matches of the finished rounds{}, Bradley-Terry first</span></p>{}",
            if compared {
                ", the entries compared by their points"
            } else {
                ""
            },
            table(
                &[
                    "*AGENT",
                    "#SEATS|the seats the agent holds, two seats of one agent count as one entry here",
                    "#MATCHES|the pairings against another agent that saw a round fought",
                    "#WON|rounds won across every pairing",
                    "#DRAWN",
                    "#LOST",
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

        let entry_rows: Vec<Vec<String>> = round
            .entries
            .iter()
            .map(|entry| {
                let agent = record.seats.get(entry.seat);
                let run =
                    runs::read(&std::path::Path::new(docker::RUN_DIRECTORY).join(&entry.run)).ok();
                let live = running.contains(&docker::scorer_container(&entry.run));
                let state = match (&run, live) {
                    (_, true) => pill(LIVE_PILL, true, "live"),
                    (Some(run), false) if run.passed() => pill(PASSED_PILL, false, "passed"),
                    (Some(run), false) if run.finished_seconds.is_some() => {
                        pill(FAILED_PILL, false, "failed")
                    }
                    (Some(_), false) => pill(BROKEN_PILL, false, "unfinished"),
                    (None, false) if playing && index + 1 == record.rounds.len() => {
                        pill(STARTING_PILL, true, "queued")
                    }
                    (None, false) => pill(BROKEN_PILL, false, "missing"),
                };
                vec![
                    (entry.seat + 1).to_string(),
                    agent
                        .map(|agent| escape(&agent.label()))
                        .unwrap_or_default(),
                    format!(
                        "<a class=\"{LINK_CLASSES}\" href=\"/run/{run}\">{run}</a>",
                        run = escape(&entry.run)
                    ),
                    state,
                    match entry.attempt {
                        Some(seconds) => format!("{seconds}s"),
                        None if live => String::new(),
                        None => "none".to_string(),
                    },
                ]
            })
            .collect();
        body.push_str(&table(
            &[
                "#SEAT",
                "*AGENT",
                "RUN",
                "STATE",
                "ENTRY|the passing push whose entry fights, by its seconds",
            ],
            entry_rows,
            None,
        ));

        let pairings = tournament::pairings(&record, round)?;
        if !pairings.is_empty() {
            body.push_str(&format!(
                "<div class=\"mt-3\">{}</div>",
                cross_table(
                    &record,
                    round,
                    &pairings,
                    ordered,
                    playing && index + 1 == record.rounds.len()
                )
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

/// The place of one agent on the leaderboard of a tournament.
struct Standing {
    agent: String,
    seats: usize,
    matches: usize,
    /// The rounds across every pairing, from the agent's view.
    tally: ava_wire::Tally,
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
            let mut tally = ava_wire::Tally::default();
            let mut played = 0;
            for pairing in &pairings {
                let first = record.seats.get(pairing.first).map(ava_wire::Agent::label);
                let second = record.seats.get(pairing.second).map(ava_wire::Agent::label);
                if first == second || pairing.tally.rounds() == 0 {
                    continue;
                }
                if first.as_deref() == Some(&agent) {
                    played += 1;
                    tally.won += pairing.tally.won;
                    tally.drawn += pairing.tally.drawn;
                    tally.lost += pairing.tally.lost;
                } else if second.as_deref() == Some(&agent) {
                    played += 1;
                    tally.won += pairing.tally.lost;
                    tally.drawn += pairing.tally.drawn;
                    tally.lost += pairing.tally.won;
                }
            }

            Standing {
                seats: record
                    .seats
                    .iter()
                    .filter(|seat| seat.label() == agent)
                    .count(),
                matches: played,
                tally,
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
fn rating_label(rating: Option<f64>) -> String {
    rating
        .map(|rating| format!("{}", rating.round() as i64))
        .unwrap_or_default()
}

/// The `pairings` of one round as a cross table: the tally of the row's seat
/// against the column's seat, and its total across the row. An `ordered`
/// playout pairs every seat with every other twice, once attacking and once
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
    headers.push("#TOTAL|rounds won, drawn and lost across the row".to_string());
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
                        cells.push(format!(
                            "{} <a class=\"{LINK_CLASSES} text-xs\" href=\"/run/{run}\">run</a>",
                            if started {
                                pill(LIVE_PILL, true, "live")
                            } else {
                                pill(STARTING_PILL, true, "queued")
                            },
                            run = escape(run)
                        ));
                    }
                    Some((tally, reason, run)) => {
                        total.won += tally.won;
                        total.drawn += tally.drawn;
                        total.lost += tally.lost;
                        cells.push(tally_cell(&tally, reason, run));
                    }
                    None => cells.push(String::new()),
                }
            }

            cells.push(tally_cell(&total, None, None));
            cells
        })
        .collect();

    table(&headers, rows, None)
}

/// A tally as `won-drawn-lost`, tinted by who came out ahead, with the reason
/// behind it as a tooltip when there is one and the run that played it linked.
fn tally_cell(tally: &ava_wire::Tally, reason: Option<&str>, run: Option<&str>) -> String {
    let tint = match tally.won.cmp(&tally.lost) {
        std::cmp::Ordering::Greater => AHEAD_CLASSES,
        std::cmp::Ordering::Less => BEHIND_CLASSES,
        std::cmp::Ordering::Equal => LEVEL_CLASSES,
    };
    let label = format!(
        "<span class=\"{MONO_CLASSES} {tint}\">{}-{}-{}</span>",
        tally.won, tally.drawn, tally.lost
    );
    let played = match run {
        Some(run) => format!(
            " <a class=\"{LINK_CLASSES} text-xs\" href=\"/run/{run}\">run</a>",
            run = escape(run)
        ),
        None => String::new(),
    };

    format!("{}{played}", explained(&label, reason.unwrap_or_default()))
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
        "<p class=\"{TITLE_CLASSES}\">limits <span class=\"{NOTE_CLASSES} font-normal\">as each backend reports them when asked: {}</span></p>",
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
/// The games a run or a tournament can be started on: every game but the ones
/// only starting as an attack on the entry of another.
pub(crate) fn startable_games() -> std::io::Result<Vec<String>> {
    Ok(games()?
        .into_iter()
        .filter(|game| ava_game::attacked_by(game).is_none())
        .collect())
}

pub(crate) fn games() -> std::io::Result<Vec<String>> {
    let mut games: Vec<String> = std::fs::read_dir(GAMES_DIRECTORY)
        .map_err(|error| at_path(GAMES_DIRECTORY, error))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join(TASK_DIRECTORY).is_dir())
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

/// The entry a run kept from the attempt at `seconds`, which is the entry file
/// of its game and nothing else.
pub(crate) fn run_entry(name: &str, seconds: &str, file: &str) -> Option<Vec<u8>> {
    let seconds: u64 = seconds.parse().ok()?;
    let directory = run_directory(name).ok()?;
    let run = runs::read(&directory).ok()?;
    let game = ava_game::find(&run.game)?;
    if file != game.entry() {
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

/// What the watcher last saw of docker: the running containers and the output
/// of every live scoring container, which holds the pushes graded so far.
struct Snapshot {
    containers: Vec<String>,
    scorer_logs: Vec<(String, String)>,
}

static SNAPSHOT: std::sync::Mutex<Snapshot> = std::sync::Mutex::new(Snapshot {
    containers: Vec::new(),
    scorer_logs: Vec::new(),
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
    let scorer_logs = containers
        .iter()
        .filter_map(|container| container.strip_prefix(docker::SCORER_CONTAINER_PREFIX))
        .map(|run| {
            (
                run.to_string(),
                container_logs(&docker::scorer_container(run)),
            )
        })
        .collect();

    *SNAPSHOT.lock().expect("the snapshot is never poisoned") = Snapshot {
        containers,
        scorer_logs,
    };
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

/// The graded pushes of a run: the record once the run is over, the output of
/// the scoring container while it is live, since that output is the very log
/// collected at the end, and that log itself for a run that broke before its
/// record was completed.
fn attempts_of(
    directory: &std::path::Path,
    name: &str,
    live: bool,
    run: &ava_wire::Run,
) -> Vec<ava_wire::Attempt> {
    if !live && !run.attempts.is_empty() {
        return run.attempts.clone();
    }

    let contents = if live {
        SNAPSHOT
            .lock()
            .expect("the snapshot is never poisoned")
            .scorer_logs
            .iter()
            .find(|(run, _)| run == name)
            .map(|(_, logs)| logs.clone())
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

/// Where a run sits in a tournament, linking the tournament.
fn placement_label(placement: &tournament::Placement) -> String {
    let role = match placement.attacking {
        Some(defender) => format!(
            "seat {} attacking seat {}",
            placement.seat + 1,
            defender + 1
        ),
        None => format!("seat {}", placement.seat + 1),
    };

    format!(
        "{role} in round {} of <a class=\"{LINK_CLASSES}\" href=\"/tournament/{name}\">{name}</a>",
        placement.round + 1,
        name = escape(&placement.tournament)
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
