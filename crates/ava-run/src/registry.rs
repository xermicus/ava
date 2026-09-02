//! The models and harnesses a benchmark run can pair into an agent.

const REGISTRY_FILE: &str = "registry.json";

const ANTHROPIC_URL: &str = "http://api.anthropic.com:8080";
const OPENAPI_URL: &str = "http://llm.substrate.dev:8080";
const CLAUDE_HARNESS: &str = "claude";
const PI_HARNESS: &str = "pi";
const OPENCODE_HARNESS: &str = "opencode";
const CODEX_HARNESS: &str = "codex";

/// Where codex reads its configuration, holding the staged provider setup.
const CODEX_CONFIG_FILE: &str = "/home/agent/.codex/config.toml";
const CODEX_API_PATH: &str = "/v1";

/// The server run for a whole benchmark, so the loop plugin prompts one
/// process instead of anything re-invoking the harness. The port is fixed
/// because the default is a random one, and the `opencode-loop` wrapper sends
/// its bootstrap request to the same port.
const OPENCODE_SERVE: [&str; 3] = ["serve", "--port", "4096"];

/// Keep the reason a run stalled in the agent log rather than in the container.
const OPENCODE_LOGS: &str = "--print-logs";
const OPENCODE_CONFIG_FILE: &str = "/home/agent/.config/opencode/opencode.json";

/// A discovery location opencode loads plugins from, holding the staged loop
/// plugin.
const OPENCODE_PLUGIN_FILE: &str = "/home/agent/.config/opencode/plugin/ralph-loop.js";
const OPENCODE_API_PATH: &str = "/v1";

/// The staged files, vendored as plain assets whose `__AVA_*__` placeholders
/// are filled by [`template`].
const CODEX_CONFIGURATION_TEMPLATE: &str = include_str!("../assets/codex-config.toml");
const OPENCODE_CONFIGURATION_TEMPLATE: &str = include_str!("../assets/opencode.json");
const OPENCODE_RALPH_LOOP_TEMPLATE: &str = include_str!("../assets/opencode-ralph-loop.js");
const PI_MODELS_TEMPLATE: &str = include_str!("../assets/pi-models.json");
const PI_RALPH_LOOP_TEMPLATE: &str = include_str!("../assets/pi-ralph-loop.js");

/// How much thinking a run asks for, weakest first.
///
/// These are the levels every harness expresses. Thinking is never turned off,
/// a benchmark measures the model as it is meant to be used.
pub const THINKING_LEVELS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

const CLAUDE_EFFORT: &str = "--effort";
const PI_THINKING: &str = "--thinking";

/// The arguments printing every event of an unattended claude run as a JSON
/// line, without which a run leaves no live log.
const CLAUDE_PRINT: [&str; 4] = ["--print", "--verbose", "--output-format", "stream-json"];

/// The command of the official loop plugin, whose stop hook re-feeds the same
/// prompt every time claude tries to stop, so one invocation works the whole run.
const CLAUDE_RALPH_LOOP: &str = "/ralph-wiggum:ralph-loop";

/// Output tokens one turn may spend, the same ceiling claude code puts on
/// its own requests. The highest thinking levels need more room above their
/// thinking budget.
const TURN_OUTPUT_CAP: u32 = 32_000;
const HIGHEST_THINKING_TURN_OUTPUT_CAP: u32 = 64_000;

/// The `max_output` of a route capped to one turn's spend.
fn turn_output(route: &Route, thinking: Option<&str>) -> u32 {
    let cap = match thinking {
        Some("xhigh" | "max") => HIGHEST_THINKING_TURN_OUTPUT_CAP,
        _ => TURN_OUTPUT_CAP,
    };

    route.max_output.min(cap)
}
const GATEWAY_PROVIDER: &str = "anthropic";
const PI_PROVIDER: &str = "ava";
const PI_MODELS_FILE: &str = "/home/agent/.pi/agent/models.json";
const PI_PROTOCOL: &str = "anthropic-messages";

/// The arguments printing every event of an unattended pi run as a JSON line.
const PI_MODE_JSON: [&str; 2] = ["--mode", "json"];

/// Where pi discovers extensions, holding the staged loop extension.
const PI_EXTENSION_FILE: &str = "/home/agent/.pi/agent/extensions/ralph-loop.ts";
const MODEL_OPTION: &str = "--model";
/// The variable holding the subscription of the anthropic backend, under the
/// same name on the host and in the sandbox.
pub const SUBSCRIPTION_TOKEN: &str = "CLAUDE_CODE_OAUTH_TOKEN";

/// The variable `ava` reads the gateway key from, which is not an Anthropic key.
pub const GATEWAY_KEY: &str = "LLM_SUBSTRATE_DEV_KEY";

/// The variable a harness reads the same key from, named the way it expects.
pub const GATEWAY_TOKEN: &str = "ANTHROPIC_AUTH_TOKEN";
const BASE_URL: &str = "ANTHROPIC_BASE_URL";

const CLAUDE_SETTINGS: [(&str, &str); 8] = [
    ("CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING", "1"),
    ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
    ("CLAUDE_CODE_ENABLE_TELEMETRY", "0"),
    ("DISABLE_AUTOUPDATER", "1"),
    ("DISABLE_TELEMETRY", "1"),
    ("OTEL_LOGS_EXPORTER", "none"),
    ("OTEL_METRICS_EXPORTER", "none"),
    ("OTEL_TRACES_EXPORTER", "none"),
];

const CLAUDE_MODEL: &str = "ANTHROPIC_MODEL";

const CLAUDE_TIER_SETTINGS: [&str; 3] = [
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
];

const MODEL_HEADER: &str = "MODEL";
const AGENT_HEADER: &str = "AGENT";
const BACKENDS_HEADER: &str = "BACKENDS";

/// A service answering the Anthropic API for one or more models.
#[derive(Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    /// Anthropic itself, reached with subscription credentials.
    Anthropic,
    /// The openapi gateway, reached with a gateway key.
    OpenApi,
}

impl Backend {
    /// The name identifying this backend in listings and the registry file.
    pub fn name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenApi => "openapi",
        }
    }

    /// The endpoint a harness must be pointed at.
    ///
    /// Every backend is reached in plain HTTP through the proxy, which
    /// terminates the request and connects onward with TLS. Nothing a sandbox
    /// sends leaves it as ciphertext, so every request stays inspectable.
    fn base_url(self) -> &'static str {
        match self {
            Self::Anthropic => ANTHROPIC_URL,
            Self::OpenApi => OPENAPI_URL,
        }
    }
}

/// The id and limits a single backend uses for a model.
#[derive(serde::Deserialize)]
pub struct Route {
    /// The service reached by this route.
    pub backend: Backend,
    /// The model id that service expects.
    pub id: String,
    /// The largest prompt the service accepts, in tokens.
    pub context_window: u32,
    /// The largest completion the service accepts, in tokens.
    ///
    /// A harness left to guess this picks a number small enough that a thinking
    /// model spends the whole allowance before it answers, which ends the turn
    /// with no tool call. The limit therefore comes from the serving provider.
    pub max_output: u32,
}

/// A model under test, independent of how it is reached.
#[derive(serde::Deserialize)]
pub struct Model {
    /// The name given on the command line.
    pub name: String,
    /// Every backend serving this model, most direct first.
    pub routes: Vec<Route>,
}

/// A harness, paired with a model to form an agent.
#[derive(serde::Deserialize)]
pub struct Harness {
    /// The name of the directory under `agents` holding its image.
    pub name: String,
    /// The backends this harness can be pointed at.
    pub backends: Vec<Backend>,
}

/// Every model and harness a benchmark run may pair.
#[derive(serde::Deserialize)]
pub struct Registry {
    /// The models a run may use.
    pub models: Vec<Model>,
    /// The harnesses a run may use.
    pub harnesses: Vec<Harness>,
}

impl Registry {
    /// Resolve `harness` and `model` into the invocation running that pairing.
    ///
    /// The route is chosen by walking the backends the harness supports in order,
    /// so a harness that can reach a model directly is not sent through a gateway.
    /// Credentials are read from the environment of this process rather than
    /// stored, so nothing secret reaches an image layer or the repository.
    ///
    /// The harness is started on `prompt` unattended and keeps itself working
    /// on it through its own loop plugin or built in equivalent, so ava never
    /// drives turns.
    pub fn invocation(
        &self,
        harness: &str,
        model: &str,
        prompt: &str,
        thinking: Option<&str>,
    ) -> std::io::Result<Invocation> {
        let model = self
            .models
            .iter()
            .find(|candidate| candidate.name == model)
            .ok_or_else(|| {
                unknown(
                    model,
                    "model",
                    self.models.iter().map(|entry| entry.name.as_str()),
                )
            })?;

        let harness = self.harness(harness)?;

        let route = harness
            .backends
            .iter()
            .find_map(|backend| model.routes.iter().find(|route| route.backend == *backend))
            .ok_or_else(|| {
                std::io::Error::other(format!(
                    "the {} harness cannot serve {}",
                    harness.name, model.name
                ))
            })?;

        log::info!(
            "{} reaches {} as {} on the {} backend",
            harness.name,
            model.name,
            route.id,
            route.backend.name()
        );

        match harness.name.as_str() {
            CLAUDE_HARNESS => claude_invocation(route, prompt, thinking),
            PI_HARNESS => pi_invocation(route, prompt, thinking),
            OPENCODE_HARNESS => opencode_invocation(route, prompt, thinking),
            CODEX_HARNESS => codex_invocation(route, prompt, thinking),
            name => Err(std::io::Error::other(format!(
                "no adapter is defined for the {name} harness"
            ))),
        }
    }

    /// The harness registered under `name`.
    pub(crate) fn harness(&self, name: &str) -> std::io::Result<&Harness> {
        self.harnesses
            .iter()
            .find(|candidate| candidate.name == name)
            .ok_or_else(|| {
                unknown(
                    name,
                    "harness",
                    self.harnesses.iter().map(|entry| entry.name.as_str()),
                )
            })
    }
}

/// Load the registry from `registry.json` in the working directory.
pub fn load() -> std::io::Result<Registry> {
    let contents = std::fs::read_to_string(REGISTRY_FILE)
        .map_err(|error| std::io::Error::other(format!("{REGISTRY_FILE}: {error}")))?;

    serde_json::from_str(&contents)
        .map_err(|error| std::io::Error::other(format!("{REGISTRY_FILE}: {error}")))
}

/// How a harness is told which model to use and how to authenticate.
#[derive(Clone)]
pub struct Invocation {
    /// The environment handed to the container.
    pub variables: Vec<(String, String)>,
    /// The arguments appended to the image entrypoint.
    pub arguments: Vec<String>,
    /// Configuration written into the container, by path and contents.
    pub files: Vec<(String, String)>,
}

/// `value` as a JSON string, quotes and escapes included.
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("a string is valid JSON")
}

/// A vendored asset with its `__AVA_*__` placeholders filled in.
///
/// A number replaces its placeholder together with the quotes around it, which
/// is what keeps the assets valid JSON on their own.
fn template(asset: &str, values: &[(&str, &str)]) -> String {
    values
        .iter()
        .fold(asset.to_string(), |text, (placeholder, value)| {
            text.replace(placeholder, value)
        })
}

/// Declare the gateway model to opencode before naming it on the command line.
///
/// opencode resolves a model against its own catalog and refuses an id it does
/// not know, so the model is added to the provider it already ships. The same
/// model serves the side agents, which would otherwise reach for a model the
/// gateway does not carry.
fn opencode_invocation(
    route: &Route,
    prompt: &str,
    thinking: Option<&str>,
) -> std::io::Result<Invocation> {
    let mut invocation = gateway_invocation(OPENCODE_HARNESS, route)?;
    let url = gateway_url(OPENCODE_HARNESS, route)?;
    let model = format!("{GATEWAY_PROVIDER}/{}", route.id);

    let configuration = template(
        OPENCODE_CONFIGURATION_TEMPLATE,
        &[
            ("__AVA_MODEL__", model.as_str()),
            ("__AVA_PROVIDER__", GATEWAY_PROVIDER),
            (
                "__AVA_BASE_URL__",
                format!("{url}{OPENCODE_API_PATH}").as_str(),
            ),
            ("__AVA_TOKEN__", GATEWAY_TOKEN),
            ("\"__AVA_MODEL_ID__\"", quoted(&route.id).as_str()),
            (
                "\"__AVA_CONTEXT__\"",
                route.context_window.to_string().as_str(),
            ),
            (
                "\"__AVA_OUTPUT__\"",
                turn_output(route, thinking).to_string().as_str(),
            ),
        ],
    );

    // The serve command reads the model from the configuration and takes no
    // model argument, so the gateway arguments are replaced.
    invocation.arguments = OPENCODE_SERVE
        .iter()
        .map(|argument| argument.to_string())
        .collect();
    invocation.files.push((
        OPENCODE_PLUGIN_FILE.to_string(),
        opencode_ralph_plugin(prompt, thinking),
    ));

    invocation.arguments.push(OPENCODE_LOGS.to_string());
    invocation
        .files
        .push((OPENCODE_CONFIG_FILE.to_string(), configuration));

    Ok(invocation)
}

/// The plugin looping opencode over the prompt with the server's own event
/// and prompt APIs, the counterpart of the claude loop plugin.
///
/// The kickoff is scheduled unawaited because the factory runs while the
/// server still bootstraps the instance, where an awaited request deadlocks.
/// Every settled turn, an errored one included, queues the prompt again with
/// the iteration named, until the run is ended from outside. The thinking
/// variant rides on every prompt and a rejected request is retried, so one
/// hiccup cannot end the loop. Only the kickoff session is prompted, since
/// the harness spawns side sessions of its own. There is no one to answer a
/// permission or question ask, so both are answered for the run to continue.
fn opencode_ralph_plugin(prompt: &str, thinking: Option<&str>) -> String {
    let variant = thinking.map(quoted).unwrap_or_else(|| "null".to_string());

    template(
        OPENCODE_RALPH_LOOP_TEMPLATE,
        &[
            ("__AVA_PROMPT__", quoted(prompt).as_str()),
            ("__AVA_VARIANT__", variant.as_str()),
        ],
    )
}

/// Ask the harness for `thinking` under the `option` naming it.
fn think(arguments: &mut Vec<String>, option: &str, thinking: Option<&str>) {
    let Some(level) = thinking else {
        return;
    };

    arguments.push(option.to_string());
    arguments.push(level.to_string());
}

/// Declare the gateway to pi as its own provider.
///
/// Naming the model under a provider pi knows nothing about keeps it out of
/// pi's catalog fallback, which would otherwise lend the model the limits of
/// whatever pi treats as the default Anthropic model.
fn pi_invocation(
    route: &Route,
    prompt: &str,
    thinking: Option<&str>,
) -> std::io::Result<Invocation> {
    let url = gateway_url(PI_HARNESS, route)?;
    let models = template(
        PI_MODELS_TEMPLATE,
        &[
            ("__AVA_PROVIDER__", PI_PROVIDER),
            ("__AVA_BASE_URL__", url),
            ("__AVA_PROTOCOL__", PI_PROTOCOL),
            ("__AVA_TOKEN__", GATEWAY_TOKEN),
            ("\"__AVA_MODEL_ID__\"", quoted(&route.id).as_str()),
            (
                "\"__AVA_CONTEXT__\"",
                route.context_window.to_string().as_str(),
            ),
            (
                "\"__AVA_OUTPUT__\"",
                turn_output(route, thinking).to_string().as_str(),
            ),
        ],
    );

    let mut arguments = vec![
        MODEL_OPTION.to_string(),
        format!("{PI_PROVIDER}/{}", route.id),
    ];
    think(&mut arguments, PI_THINKING, thinking);
    arguments.extend(PI_MODE_JSON.iter().map(|argument| argument.to_string()));
    arguments.push(prompt.to_string());

    Ok(Invocation {
        variables: vec![(GATEWAY_TOKEN.to_string(), credential(GATEWAY_KEY)?)],
        arguments,
        files: vec![
            (PI_MODELS_FILE.to_string(), models),
            (PI_EXTENSION_FILE.to_string(), pi_ralph_extension(prompt)),
        ],
    })
}

/// The extension looping pi over the prompt with pi's own event and message
/// APIs, the counterpart of the claude loop plugin.
///
/// A follow up queued at `agent_end` keeps the run from settling, so the
/// single invocation lasts until the run is ended from outside. An end pi
/// retries by itself is left alone; every other end, errors included, is
/// re-prompted with the iteration named.
fn pi_ralph_extension(prompt: &str) -> String {
    template(
        PI_RALPH_LOOP_TEMPLATE,
        &[("__AVA_PROMPT__", quoted(prompt).as_str())],
    )
}

/// Point codex at the gateway, looped from the outside.
///
/// Codex has no hook able to re-prompt a session, so the image wraps it in
/// `codex-loop`, which resumes the recorded session between turns. The
/// reasoning effort travels in the configuration; codex has no per turn
/// output knob, so the turn cap goes unenforced here.
fn codex_invocation(
    route: &Route,
    prompt: &str,
    thinking: Option<&str>,
) -> std::io::Result<Invocation> {
    let url = gateway_url(CODEX_HARNESS, route)?;
    let effort = match thinking {
        Some("max") => "xhigh",
        Some(level) => level,
        None => "medium",
    };

    let configuration = template(
        CODEX_CONFIGURATION_TEMPLATE,
        &[
            ("__AVA_MODEL_ID__", route.id.as_str()),
            (
                "__AVA_BASE_URL__",
                format!("{url}{CODEX_API_PATH}").as_str(),
            ),
            ("__AVA_EFFORT__", effort),
            (
                "\"__AVA_CONTEXT__\"",
                route.context_window.to_string().as_str(),
            ),
        ],
    );

    Ok(Invocation {
        variables: vec![(GATEWAY_TOKEN.to_string(), credential(GATEWAY_KEY)?)],
        arguments: vec![prompt.to_string()],
        files: vec![(CODEX_CONFIG_FILE.to_string(), configuration)],
    })
}

/// Point a harness at the gateway and name the model the way it expects.
///
/// Both third party harnesses take the Anthropic provider slot and accept an
/// unknown model id under it, which is how a gateway model reaches them.
fn gateway_url(harness: &str, route: &Route) -> std::io::Result<&'static str> {
    if route.backend != Backend::OpenApi {
        return Err(std::io::Error::other(format!(
            "the {harness} harness reaches models only through the gateway"
        )));
    }

    Ok(route.backend.base_url())
}

fn gateway_invocation(harness: &str, route: &Route) -> std::io::Result<Invocation> {
    let url = gateway_url(harness, route)?;

    Ok(Invocation {
        variables: vec![
            (BASE_URL.to_string(), url.to_string()),
            (GATEWAY_TOKEN.to_string(), credential(GATEWAY_KEY)?),
        ],
        arguments: vec![
            MODEL_OPTION.to_string(),
            format!("{GATEWAY_PROVIDER}/{}", route.id),
        ],
        files: Vec::new(),
    })
}

fn claude_invocation(
    route: &Route,
    prompt: &str,
    thinking: Option<&str>,
) -> std::io::Result<Invocation> {
    let mut environment: Vec<(String, String)> = CLAUDE_SETTINGS
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();

    environment.push((BASE_URL.to_string(), route.backend.base_url().to_string()));

    match route.backend {
        Backend::Anthropic => {
            environment.push((CLAUDE_MODEL.to_string(), route.id.clone()));
            environment.push((
                SUBSCRIPTION_TOKEN.to_string(),
                credential(SUBSCRIPTION_TOKEN)?,
            ));
        }
        Backend::OpenApi => {
            for name in CLAUDE_TIER_SETTINGS {
                environment.push((name.to_string(), route.id.clone()));
            }
            environment.push((GATEWAY_TOKEN.to_string(), credential(GATEWAY_KEY)?));
        }
    }

    let mut arguments = Vec::new();
    think(&mut arguments, CLAUDE_EFFORT, thinking);
    arguments.extend(CLAUDE_PRINT.iter().map(|argument| argument.to_string()));
    arguments.push(format!("{CLAUDE_RALPH_LOOP} {prompt}"));

    Ok(Invocation {
        variables: environment,
        arguments,
        files: Vec::new(),
    })
}

/// The kind marking a credential the environment does not carry, telling a
/// deployment nobody finished apart from a pairing that cannot work.
const MISSING_CREDENTIAL: std::io::ErrorKind = std::io::ErrorKind::NotFound;

/// Whether `error` reports a credential the environment does not carry.
pub fn is_missing_credential(error: &std::io::Error) -> bool {
    error.kind() == MISSING_CREDENTIAL
}

fn credential(variable: &str) -> std::io::Result<String> {
    std::env::var(variable).map_err(|_| {
        std::io::Error::new(
            MISSING_CREDENTIAL,
            format!("{variable} is not set in the environment"),
        )
    })
}

/// The error naming what is known instead of the `given` unknown `kind`.
pub(crate) fn unknown<'a>(
    given: &str,
    kind: &str,
    known: impl Iterator<Item = &'a str>,
) -> std::io::Error {
    std::io::Error::other(format!(
        "unknown {kind} `{given}`, known are: {}",
        known.collect::<Vec<_>>().join(", ")
    ))
}

/// Load the registry, or report the failure and exit(1).
fn load_or_exit() -> Registry {
    load().unwrap_or_else(|error| {
        eprintln!("error: {error}");
        std::process::exit(1);
    })
}

/// Print every known model with the backend serving it, then exit.
pub fn list_models() -> ! {
    let registry = load_or_exit();
    let names = registry.models.iter().map(|model| model.name.as_str());
    let width = column_width(MODEL_HEADER, names);

    println!("{MODEL_HEADER:<width$}  {BACKENDS_HEADER}");
    for model in &registry.models {
        let backends: Vec<&str> = model
            .routes
            .iter()
            .map(|route| route.backend.name())
            .collect();
        println!("{:<width$}  {}", model.name, backends.join(", "));
    }
    std::process::exit(0);
}

/// Print every known agent with the backends it can be paired with, then exit.
pub fn list_agents() -> ! {
    let registry = load_or_exit();
    let names = registry.harnesses.iter().map(|agent| agent.name.as_str());
    let width = column_width(AGENT_HEADER, names);

    println!("{AGENT_HEADER:<width$}  {BACKENDS_HEADER}");
    for agent in &registry.harnesses {
        let backends: Vec<&str> = agent
            .backends
            .iter()
            .map(|backend| backend.name())
            .collect();
        println!("{:<width$}  {}", agent.name, backends.join(", "));
    }
    std::process::exit(0);
}

fn column_width<'a>(header: &str, entries: impl Iterator<Item = &'a str>) -> usize {
    entries
        .map(|entry| entry.len())
        .chain(std::iter::once(header.len()))
        .max()
        .unwrap_or_default()
}
