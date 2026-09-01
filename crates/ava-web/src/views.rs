//! The pages of the web interface, rendered from the run artifacts on disk,
//! the registry and the games folder.

use ava_run::{docker, process, registry};

const GAMES_DIRECTORY: &str = "games";
const TASK_FILE: &str = "task.md";
const INSTRUCTIONS_FILE: &str = "README.md";

/// Every game scores within this ceiling, which is what makes one point
/// bar comparable across all of them.
const POINT_CEILING: u64 = 10000;

/// The width of a bar, in block characters.
const BAR_CELLS: u64 = 10;
const BAR_FULL: char = '\u{2588}';
const BAR_EMPTY: char = '\u{2591}';

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
const TITLE_PLACEHOLDER: &str = "__AVA_TITLE__";
const BODY_PLACEHOLDER: &str = "__AVA_BODY__";

/// A `#` prefix on a header right-aligns that column for numbers.
const NUMERIC_MARKER: char = '#';

const TABLE_CLASSES: &str = "border-collapse";
const HEADER_CLASSES: &str = "text-left text-neutral-500 font-normal py-1.5 pr-6";
const CELL_CLASSES: &str = "py-1 pr-6 border-t border-neutral-200 align-top";
const ROW_CLASSES: &str = "hover:bg-neutral-50";
const TITLE_CLASSES: &str = "font-medium mt-10 mb-2";
const NOTE_CLASSES: &str = "text-neutral-500";
const CONSOLE_CLASSES: &str = "text-xs whitespace-pre-wrap break-all \
     bg-neutral-50 border border-neutral-200 p-3 overflow-x-auto";
const LINK_CLASSES: &str =
    "underline decoration-neutral-300 underline-offset-2 hover:decoration-neutral-900";
const BUTTON_CLASSES: &str = "bg-green-700 text-white px-4 py-1 hover:bg-green-800";
const STOP_CLASSES: &str = "border border-red-700 text-red-700 px-2 hover:bg-red-50";
const FIELD_CLASSES: &str = "border border-neutral-400 bg-white px-2 py-1 w-full";
const LABEL_CLASSES: &str = "block text-neutral-500 mb-1";

/// What the last action left for the page to show.
pub(crate) struct Notice {
    /// The action went ahead.
    pub started: Option<String>,
    /// The action was refused, and why.
    pub refused: Option<String>,
}

impl Notice {
    /// The notice as an inline line, colored by outcome.
    fn render(&self) -> String {
        if let Some(refused) = &self.refused {
            return format!("<p class=\"text-red-700 mt-4\">{}</p>", escape(refused));
        }

        match &self.started {
            Some(action) => format!("<p class=\"text-green-700 mt-4\">{}</p>", escape(action)),
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
        format!("{} {}", escape(self.harness()), escape(self.thinking()))
            .trim_end()
            .to_string()
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

    /// The names of the variables the sandbox was given, attributing the
    /// run to the credential feeding it.
    fn variables(&self) -> Vec<&str> {
        self.metadata
            .get("variables")
            .and_then(serde_json::Value::as_array)
            .map(|items| items.iter().filter_map(serde_json::Value::as_str).collect())
            .unwrap_or_default()
    }

    /// One aggregated number out of the proxy metrics of the run.
    fn metric(&self, key: &str) -> u64 {
        pointer(self.score.as_ref(), &format!("/metrics/{key}"))
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

    /// The state of the run as a marked, colored label.
    fn state(&self) -> String {
        let (class, label) = if self.live {
            (
                "text-green-700",
                "<span class=\"animate-pulse\">\u{25cf}</span> live",
            )
        } else if self.solved() {
            ("text-green-700", "\u{2713} solved")
        } else if self.score.is_some() {
            ("text-neutral-500", "\u{00b7} unsolved")
        } else {
            ("text-red-700", "\u{2715} no score")
        };

        format!("<span class=\"{class}\">{label}</span>")
    }

    /// How far into its time budget a live run is, as a bar.
    ///
    /// The heartbeat of the run loop is the elapsed time of record: the loop
    /// clock pauses with a sleeping host, which wall clock arithmetic misses.
    fn elapsed_bar(&self) -> String {
        let elapsed = match &self.monitor {
            Some(heartbeat) => number(heartbeat, "elapsed_seconds"),
            None => epoch_now().saturating_sub(self.started()),
        }
        .min(self.limit());

        format!("{} {elapsed}/{}s", bar(elapsed, self.limit()), self.limit())
    }

    /// What the agent printed so far, for a live run.
    fn output(&self) -> String {
        match &self.monitor {
            Some(heartbeat) => format!("{} KiB", number(heartbeat, "output_bytes") / 1024),
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
}

/// The landing page: what runs now, the start panel, and the history.
pub(crate) fn runs_page(
    notice: &Notice,
    selection: &Selection,
    pending: &[Pending],
) -> std::io::Result<String> {
    let runs = collect_runs()?;

    let mut live_rows: Vec<Vec<String>> = Vec::new();

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
            live_rows.push(vec![
                format!(
                    "<span class=\"{NOTE_CLASSES}\"><span class=\"animate-pulse\">\u{25cc}</span> starting</span>"
                ),
                escape(&start.game),
                escape(&start.model),
                format!("{} {}", escape(&start.agent), escape(&start.thinking))
                    .trim_end()
                    .to_string(),
                format!("asked {} ago", age(start.started)),
                String::new(),
                String::new(),
            ]);
        }
    }

    live_rows.extend(runs.iter().filter(|run| run.live).map(|run| {
        vec![
            run.link(),
            escape(run.game()),
            escape(run.model()),
            run.agent(),
            run.elapsed_bar(),
            run.output(),
            run.stop_form(),
        ]
    }));

    let history_rows: Vec<Vec<String>> = runs
        .iter()
        .filter(|run| !run.live)
        .map(|run| {
            vec![
                run.link(),
                run.state(),
                escape(run.game()),
                escape(run.model()),
                run.agent(),
                run.wall.map(|wall| format!("{wall}s")).unwrap_or_default(),
                format!("{}s", run.limit()),
                age(run.started()),
                run.attempts().to_string(),
                points_bar(run.points()),
            ]
        })
        .collect();

    let mut body = start_panel(selection)?;
    body.push_str("<div data-refresh=\"live\">");
    body.push_str(&notice.render());

    if !live_rows.is_empty() {
        body.push_str(&format!(
            "<p class=\"{TITLE_CLASSES} text-green-700\">live</p>{}",
            table(
                &["RUN", "GAME", "MODEL", "HARNESS", "ELAPSED", "#OUTPUT", ""],
                live_rows
            )
        ));
    }
    body.push_str("</div>");

    body.push_str(&format!(
        "<details class=\"mt-10\"><summary class=\"cursor-pointer font-medium\">history</summary>\
         <div data-refresh=\"history\"><p class=\"{NOTE_CLASSES} mt-2 mb-2\">{} runs</p>{}</div></details>",
        history_rows.len(),
        table(
            &[
                "RUN", "STATE", "GAME", "MODEL", "HARNESS", "#TIME", "#LIMIT", "#AGE", "#PUSHES",
                "POINTS"
            ],
            history_rows
        )
    ));

    Ok(page("AvA", &body))
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
        "<div class=\"border border-neutral-300 mt-8 p-4 max-w-fit\">\
         <p class=\"font-medium mb-3\">new run</p>\
         <form method=\"post\" action=\"/start\" class=\"flex flex-wrap items-end gap-3\">\
         {}{}{}{}\
         <label class=\"w-24\"><span class=\"{LABEL_CLASSES}\">seconds</span>\
         <input class=\"{FIELD_CLASSES}\" type=\"number\" name=\"limit\" value=\"{limit}\" min=\"1\"></label>\
         <label class=\"w-20\"><span class=\"{LABEL_CLASSES}\">parallel</span>\
         <input class=\"{FIELD_CLASSES}\" type=\"number\" name=\"parallel\" value=\"{parallel}\" min=\"1\"></label>\
         <label class=\"flex items-center gap-2 py-1\">\
         <input type=\"checkbox\" name=\"force\" class=\"accent-green-700\">\
         <span class=\"{NOTE_CLASSES}\">rebuild images</span></label>\
         <button class=\"{BUTTON_CLASSES}\">start</button>\
         </form></div>",
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
fn select(name: &str, options: &[&str], selected: &str) -> String {
    let mut rendered = format!(
        "<label class=\"w-44\"><span class=\"{LABEL_CLASSES}\">{name}</span>\
         <select class=\"{FIELD_CLASSES}\" name=\"{name}\">"
    );
    for option in options {
        let marked = if *option == selected { " selected" } else { "" };
        rendered.push_str(&format!("<option{marked}>{}</option>", escape(option)));
    }
    rendered.push_str("</select></label>");
    rendered
}

/// One run: its state and score first, the pushes, the console, then the rest.
pub(crate) fn run_page(name: &str, notice: &Notice) -> std::io::Result<String> {
    let directory = run_directory(name)?;

    let metadata = read_json(&directory.join(docker::METADATA_FILE)).unwrap_or_default();
    let score = read_json(&directory.join(docker::SCORE_FILE));
    let entry = RunEntry {
        name: name.to_string(),
        live: live_runs()
            .iter()
            .any(|running| running == &docker::sandbox_container(name)),
        wall: wall_seconds(&directory, number(&metadata, "started_seconds")),
        monitor: read_json(&directory.join(docker::MONITOR_FILE)),
        metadata,
        score,
    };

    let mut body = format!(
        "<div data-refresh=\"run\"><div class=\"flex items-baseline gap-4 mt-8\">\
         <h1 class=\"font-medium text-lg\">{}</h1>{}{}</div>",
        escape(name),
        entry.state(),
        entry.stop_form()
    );
    body.push_str(&notice.render());

    let took = match entry.wall.filter(|_| !entry.live) {
        Some(wall) => format!("{wall}s of "),
        None => String::new(),
    };
    body.push_str(&format!(
        "<p class=\"{NOTE_CLASSES} mt-1\">{} \u{00b7} {} \u{00b7} {} \u{00b7} {took}{}s budget \u{00b7} started {} ago</p>",
        escape(entry.game()),
        escape(entry.model()),
        entry.agent(),
        entry.limit(),
        age(entry.started()),
    ));

    if entry.live {
        body.push_str(&format!("<p class=\"mt-3\">{}</p>", entry.elapsed_bar()));
    }

    if entry.score.is_some() {
        let outcome = if entry.solved() {
            format!(", solved at {}s", entry.seconds())
        } else {
            ", unsolved".to_string()
        };
        body.push_str(&format!(
            "<p class=\"mt-3\">{} points{outcome}, {} pushes</p>",
            points_bar(entry.points()),
            entry.attempts()
        ));
    }

    let pushes = push_rows(&directory.join(docker::SCORE_LOG));
    if !pushes.is_empty() {
        body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">pushes</p>"));
        body.push_str(&table(&["#SECONDS", "STATE", "POINTS", "REASON"], pushes));
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
        "<details class=\"mt-10\"><summary class=\"cursor-pointer {NOTE_CLASSES}\">parameters</summary>{}</details>",
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
                points_bar(standing.points),
                standing.seconds.to_string(),
            ]
        })
        .collect();

    let body = format!(
        "<p class=\"{TITLE_CLASSES}\">scoreboard <span class=\"{NOTE_CLASSES} font-normal\">the best run of every pairing</span></p>{}",
        table(
            &[
                "GAME", "MODEL", "HARNESS", "#RUNS", "#SOLVED", "BEST", "#SECONDS",
            ],
            rows,
        )
    );

    Ok(page("AvA scoreboard", &body))
}

/// Every game: its task, its record and its standings.
pub(crate) fn games_page() -> std::io::Result<String> {
    let runs = collect_runs()?;
    let mut body = String::new();

    for game in games()? {
        let played: Vec<&RunEntry> = runs.iter().filter(|run| run.game() == game).collect();
        let solved = played.iter().filter(|run| run.solved()).count();
        let mut standing: Vec<&&RunEntry> = played
            .iter()
            .filter(|run| run.solved() && run.comparable())
            .collect();
        standing.sort_by_key(|run| std::cmp::Reverse(run.points()));

        body.push_str(&format!(
            "<p class=\"font-medium mt-12\">{}</p>",
            escape(&game)
        ));

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
        body.push_str(&format!("<p class=\"{NOTE_CLASSES} mt-1\">{record}</p>"));

        let task = std::fs::read_to_string(
            std::path::Path::new(GAMES_DIRECTORY)
                .join(&game)
                .join(TASK_FILE),
        )
        .unwrap_or_default();
        body.push_str(&format!(
            "<div class=\"mt-2\">{}</div>",
            crate::markdown::render(&task)
        ));

        let standings: Vec<Vec<String>> = standing
            .iter()
            .take(STANDINGS_LIMIT)
            .map(|run| {
                vec![
                    escape(run.model()),
                    run.agent(),
                    points_bar(run.points()),
                    run.seconds().to_string(),
                    run.link(),
                ]
            })
            .collect();
        if !standings.is_empty() {
            body.push_str(&table(
                &["MODEL", "HARNESS", "POINTS", "#SECONDS", "RUN"],
                standings,
            ));
        }
    }

    if let Ok(instructions) =
        std::fs::read_to_string(std::path::Path::new(GAMES_DIRECTORY).join(INSTRUCTIONS_FILE))
    {
        body.push_str(&format!(
            "<details class=\"mt-12\"><summary class=\"cursor-pointer {NOTE_CLASSES}\">the instructions shared by every game</summary>{}</details>",
            crate::markdown::render(&instructions)
        ));
    }

    Ok(page("AvA games", &body))
}

/// The registry, the credentials and the docker images runs are built from.
pub(crate) fn setup_page() -> std::io::Result<String> {
    let registry = registry::load()?;
    let runs = collect_runs()?;

    let mut key_rows = Vec::new();
    let mut limit_lines = String::new();
    for (backend, variable, sandbox_variable) in [
        (
            "anthropic",
            registry::SUBSCRIPTION_TOKEN,
            registry::SUBSCRIPTION_TOKEN,
        ),
        ("openapi", registry::GATEWAY_KEY, registry::GATEWAY_TOKEN),
    ] {
        let state = if std::env::var(variable).is_ok() {
            "<span class=\"text-green-700\">\u{2713} set</span>"
        } else {
            "<span class=\"text-red-700\">\u{2715} missing</span>"
        };

        // A subscription run carries its token under the same name, so the
        // gateway row only counts runs without one.
        let fed: Vec<&RunEntry> = runs
            .iter()
            .filter(|run| {
                let variables = run.variables();
                variables.contains(&sandbox_variable)
                    && (variable == registry::SUBSCRIPTION_TOKEN
                        || !variables.contains(&registry::SUBSCRIPTION_TOKEN))
            })
            .collect();

        let mut row = vec![variable.to_string(), backend.to_string(), state.to_string()];
        row.push(fed.len().to_string());
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
        key_rows.push(row);

        // The runs are newest first, so the first captured set is the
        // freshest view of the account.
        if let Some(run) = fed
            .iter()
            .find(|run| !run.metric_text("ratelimits").is_empty())
        {
            limit_lines.push_str(&format!(
                "<p class=\"{NOTE_CLASSES} mt-2 max-w-4xl\">{backend} limits, reported {} ago: <span class=\"text-neutral-900\">{}</span></p>",
                age(run.started()),
                escape(run.metric_text("ratelimits"))
            ));
        }
    }

    let mut model_rows = Vec::new();
    for model in &registry.models {
        for route in &model.routes {
            model_rows.push(vec![
                escape(&model.name),
                route.backend.name().to_string(),
                escape(&route.id),
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
                    .backends
                    .iter()
                    .map(|backend| backend.name())
                    .collect::<Vec<_>>()
                    .join(" "),
            ]
        })
        .collect();

    let mut body = format!(
        "<p class=\"{TITLE_CLASSES}\">keys <span class=\"{NOTE_CLASSES} font-normal\">with the usage recorded over every run on disk</span></p>"
    );
    body.push_str(&table(
        &[
            "KEY",
            "BACKEND",
            "STATE",
            "#RUNS",
            "#REQUESTS",
            "#INPUT",
            "#OUTPUT",
            "#CACHE READ",
            "#CACHE WRITE",
        ],
        key_rows,
    ));
    body.push_str(&limit_lines);
    body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">models</p>"));
    body.push_str(&table(
        &["MODEL", "BACKEND", "ID", "#CONTEXT", "#MAX OUTPUT"],
        model_rows,
    ));
    body.push_str(&format!("<p class=\"{TITLE_CLASSES}\">harnesses</p>"));
    body.push_str(&table(&["HARNESS", "BACKENDS"], harness_rows));

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
        body.push_str(&table(&["IMAGE", "TAG", "SIZE", "CREATED"], rows));
    }

    Ok(page("AvA setup", &body))
}

/// A page carrying one message, for the errors of the reading views.
pub(crate) fn message_page(title: &str, message: &str) -> String {
    let body = format!(
        "<p class=\"mt-10 max-w-prose\">{}</p><p class=\"mt-4\"><a class=\"{LINK_CLASSES}\" href=\"/\">back to the runs</a></p>",
        escape(message)
    );

    page(title, &body)
}

/// The known game folders, sorted.
pub(crate) fn games() -> std::io::Result<Vec<String>> {
    let mut games: Vec<String> = std::fs::read_dir(GAMES_DIRECTORY)?
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
fn collect_runs() -> std::io::Result<Vec<RunEntry>> {
    let running = live_runs();

    let mut runs = Vec::new();
    for entry in std::fs::read_dir(docker::RUN_DIRECTORY)?.filter_map(Result::ok) {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Some(metadata) = read_json(&entry.path().join(docker::METADATA_FILE)) else {
            continue;
        };

        let sandbox = docker::sandbox_container(&name);
        runs.push(RunEntry {
            live: running.iter().any(|container| container == &sandbox),
            score: read_json(&entry.path().join(docker::SCORE_FILE)),
            wall: wall_seconds(&entry.path(), number(&metadata, "started_seconds")),
            monitor: read_json(&entry.path().join(docker::MONITOR_FILE)),
            metadata,
            name,
        });
    }

    runs.sort_by_key(|run| std::cmp::Reverse(number(&run.metadata, "started_seconds")));

    Ok(runs)
}

/// The pushes of `score.log` as table rows.
fn push_rows(log: &std::path::Path) -> Vec<Vec<String>> {
    let Ok(contents) = std::fs::read_to_string(log) else {
        return Vec::new();
    };

    contents
        .lines()
        .filter(|line| line.starts_with('{'))
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .map(|push| {
            let solved = push
                .get("solved")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            vec![
                number(&push, "seconds").to_string(),
                if solved {
                    "<span class=\"text-green-700\">\u{2713} solved</span>".to_string()
                } else {
                    "<span class=\"text-neutral-500\">\u{00b7} unsolved</span>".to_string()
                },
                points_bar(number(&push, "points")),
                escape(text(&push, "reason")),
            ]
        })
        .collect()
}

/// A flat JSON object as a two column table.
fn object_table(value: &serde_json::Value) -> String {
    let Some(object) = value.as_object() else {
        return String::new();
    };

    let mut html = format!("<table class=\"{TABLE_CLASSES}\"><tbody>");
    for (key, value) in object {
        html.push_str(&format!(
            "<tr><td class=\"{CELL_CLASSES} text-neutral-500 w-56\">{}</td><td class=\"{CELL_CLASSES}\">{}</td></tr>",
            escape(key),
            escape(&plain(value))
        ));
    }
    html.push_str("</tbody></table>");
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

/// A points value behind its bar on the shared 0 to 10000 scale.
fn points_bar(points: u64) -> String {
    format!("{} {points}", bar(points.min(POINT_CEILING), POINT_CEILING))
}

/// `value` out of `ceiling` as a fixed width bar of block characters.
fn bar(value: u64, ceiling: u64) -> String {
    let filled = (value.min(ceiling) * BAR_CELLS + ceiling / 2)
        .checked_div(ceiling)
        .unwrap_or(0);

    format!(
        "<span class=\"text-green-700\">{}</span><span class=\"text-neutral-300\">{}</span>",
        BAR_FULL.to_string().repeat(filled as usize),
        BAR_EMPTY.to_string().repeat((BAR_CELLS - filled) as usize)
    )
}

/// A table whose `#` marked headers hold right-aligned numbers.
fn table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let numeric: Vec<bool> = headers
        .iter()
        .map(|header| header.starts_with(NUMERIC_MARKER))
        .collect();

    let mut html = format!("<table class=\"{TABLE_CLASSES}\"><thead><tr>");
    for (header, numeric) in headers.iter().zip(&numeric) {
        let align = if *numeric { " text-right" } else { "" };
        html.push_str(&format!(
            "<th class=\"{HEADER_CLASSES}{align}\">{}</th>",
            header.trim_start_matches(NUMERIC_MARKER)
        ));
    }
    html.push_str("</tr></thead><tbody>");

    for row in rows {
        html.push_str(&format!("<tr class=\"{ROW_CLASSES}\">"));
        for (cell, numeric) in row.iter().zip(&numeric) {
            let align = if *numeric { " text-right" } else { "" };
            html.push_str(&format!("<td class=\"{CELL_CLASSES}{align}\">{cell}</td>"));
        }
        html.push_str("</tr>");
    }

    html.push_str("</tbody></table>");
    html
}

/// The layout around one rendered `body`.
fn page(title: &str, body: &str) -> String {
    LAYOUT_TEMPLATE
        .replace(TITLE_PLACEHOLDER, &escape(title))
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
