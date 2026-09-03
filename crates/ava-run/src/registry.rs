//! The backends, models and harnesses a benchmark run can pair into an agent.

const REGISTRY_FILE: &str = "registry.json";

/// The port the bridge listens on inside the sandbox, forwarding every host
/// pinned to loopback onto the proxy socket.
const PROXY_PORT: u16 = 8080;

const CLAUDE_HARNESS: &str = "claude";
const PI_HARNESS: &str = "pi";
const OPENCODE_HARNESS: &str = "opencode";
const CODEX_HARNESS: &str = "codex";

/// Where codex reads its configuration, holding the staged provider setup.
const CODEX_CONFIG_FILE: &str = "/home/agent/.codex/config.toml";
const CODEX_API_PATH: &str = "/v1";

/// Keep the reason a run stalled in the agent log rather than in the container.
const OPENCODE_LOGS: &str = "--print-logs";
const OPENCODE_CONFIG_FILE: &str = "/home/agent/.config/opencode/opencode.json";

const OPENCODE_API_PATH: &str = "/v1";

/// The staged files, vendored as plain assets whose `__AVA_*__` placeholders
/// are filled by [`template`].
const CODEX_CONFIGURATION_TEMPLATE: &str = include_str!("../assets/codex-config.toml");
const OPENCODE_CONFIGURATION_TEMPLATE: &str = include_str!("../assets/opencode.json");
const PI_MODELS_TEMPLATE: &str = include_str!("../assets/pi-models.json");

/// What one start of a harness is: the turn that opens the session, or a turn
/// continuing the session the start before it left behind.
///
/// Every turn of a run is a start of its own. `ava` prompts the harness, the
/// harness answers one turn and exits, and the exit is the turn boundary `ava`
/// starts the next turn on. No harness loops itself, so all four take the same
/// path and the loop is one thing rather than four.
#[derive(Clone, Copy, PartialEq)]
pub enum Start {
    /// The first turn, which opens the session.
    Task,
    /// A later turn, continuing the recorded session.
    Resume,
}

/// The options resuming the recorded session of each harness.
const CLAUDE_CONTINUE: &str = "--continue";
const PI_CONTINUE: &str = "--continue";
const OPENCODE_CONTINUE: &str = "--continue";
const CODEX_EXEC: [&str; 2] = ["exec", "--json"];
const CODEX_RESUME: [&str; 4] = ["exec", "resume", "--last", "--json"];

/// The arguments printing every event of an unattended opencode run as a JSON line.
const OPENCODE_RUN: [&str; 4] = ["run", "--auto", "--format", "json"];

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

const MODEL_OPTION: &str = "--model";

/// The variable claude reads a subscription from inside the sandbox.
const SUBSCRIPTION_TOKEN: &str = "CLAUDE_CODE_OAUTH_TOKEN";

/// The variable a harness reads a gateway key from inside the sandbox, named
/// the way it expects.
const GATEWAY_TOKEN: &str = "ANTHROPIC_AUTH_TOKEN";
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
const SERVICES_HEADER: &str = "SERVICES";

/// A service answering the Anthropic API for one or more models.
#[derive(Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Service {
    /// Anthropic itself, reached with subscription credentials.
    Anthropic,
    /// An openapi gateway, reached with a gateway key.
    OpenApi,
}

impl Service {
    /// The name identifying this service in listings and the registry file.
    pub fn name(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenApi => "openapi",
        }
    }
}

/// A backend declared in the registry, named by the routes it serves.
#[derive(serde::Deserialize)]
pub struct Backend {
    /// The name a route refers to the backend by.
    pub name: String,
    /// The service answering at the host.
    pub service: Service,
    /// The host the proxy forwards to, which the sandbox resolves to loopback.
    pub host: String,
    /// The environment variable holding the credential of the backend.
    pub key: String,
}

impl Backend {
    /// The endpoint a harness is pointed at.
    ///
    /// Every backend is reached in plain HTTP through the proxy, which
    /// terminates the request and connects onward with TLS. Nothing a sandbox
    /// sends leaves it as ciphertext, so every request stays inspectable.
    fn url(&self) -> String {
        format!("http://{}:{PROXY_PORT}", self.host)
    }

    /// The credential of the backend, read from the environment of this
    /// process rather than stored, so nothing secret reaches an image layer or
    /// the repository.
    pub(crate) fn credential(&self) -> std::io::Result<String> {
        credential(&self.key)
    }
}

/// The id and limits a single backend uses for a model.
#[derive(serde::Deserialize)]
pub struct Route {
    /// The name of the backend serving this route.
    pub backend: String,
    /// The model id that backend expects.
    pub id: String,
    /// The largest prompt the backend accepts, in tokens.
    pub context_window: u32,
    /// The largest completion the backend accepts, in tokens.
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
    /// The services this harness can be pointed at, most direct first.
    pub services: Vec<Service>,
}

/// Every backend, model and harness a benchmark run may pair.
#[derive(serde::Deserialize)]
pub struct Registry {
    /// The backends the routes of the models name.
    pub backends: Vec<Backend>,
    /// The models a run may use.
    pub models: Vec<Model>,
    /// The harnesses a run may use.
    pub harnesses: Vec<Harness>,
}

impl Registry {
    /// Resolve `harness` and `model` into the invocation running that pairing.
    ///
    /// The route is chosen by walking the services the harness speaks in order
    /// and taking the first route of the model on such a service, so a harness
    /// that can reach a model directly is not sent through a gateway.
    /// Credentials are read from the environment of this process rather than
    /// stored, so nothing secret reaches an image layer or the repository.
    ///
    /// Every turn of a run is one invocation: the first opens the session on
    /// `prompt` and every later one resumes the recorded session on it.
    pub fn invocation(
        &self,
        harness: &str,
        model: &str,
        prompt: &str,
        thinking: Option<&str>,
        start: Start,
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

        let mut served: Vec<(&Route, &Backend)> = Vec::new();
        for route in &model.routes {
            served.push((route, self.backend(&route.backend)?));
        }

        let (route, backend) = harness
            .services
            .iter()
            .find_map(|service| {
                served
                    .iter()
                    .find(|(_, backend)| backend.service == *service)
            })
            .copied()
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
            backend.name
        );

        let mut invocation = match harness.name.as_str() {
            CLAUDE_HARNESS => claude_invocation(route, backend, prompt, thinking, start),
            PI_HARNESS => pi_invocation(route, backend, prompt, thinking, start),
            OPENCODE_HARNESS => opencode_invocation(route, backend, prompt, thinking, start),
            CODEX_HARNESS => codex_invocation(route, backend, prompt, thinking, start),
            name => Err(std::io::Error::other(format!(
                "no adapter is defined for the {name} harness"
            ))),
        }?;

        invocation.hosts = self.hosts();

        Ok(invocation)
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

    /// The backend registered under `name`.
    fn backend(&self, name: &str) -> std::io::Result<&Backend> {
        self.backends
            .iter()
            .find(|candidate| candidate.name == name)
            .ok_or_else(|| {
                unknown(
                    name,
                    "backend",
                    self.backends.iter().map(|entry| entry.name.as_str()),
                )
            })
    }

    /// Every distinct host a registered backend is reached at, in registry
    /// order.
    ///
    /// This is the single source for the nginx allowlist and the sandbox host
    /// entries.
    pub fn hosts(&self) -> Vec<String> {
        let mut hosts: Vec<String> = Vec::new();

        for backend in &self.backends {
            if !hosts.contains(&backend.host) {
                hosts.push(backend.host.clone());
            }
        }

        hosts
    }
}

/// Load the registry from `registry.json` in the working directory, checking
/// that every route names a registered backend.
pub fn load() -> std::io::Result<Registry> {
    let contents = std::fs::read_to_string(REGISTRY_FILE)
        .map_err(|error| std::io::Error::other(format!("{REGISTRY_FILE}: {error}")))?;

    let registry: Registry = serde_json::from_str(&contents)
        .map_err(|error| std::io::Error::other(format!("{REGISTRY_FILE}: {error}")))?;

    for model in &registry.models {
        for route in &model.routes {
            registry.backend(&route.backend).map_err(|error| {
                std::io::Error::other(format!("{REGISTRY_FILE}: {}: {error}", model.name))
            })?;
        }
    }

    Ok(registry)
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
    /// The hosts the sandbox resolves to loopback, where the bridge forwards
    /// them onto the proxy: every host a registered backend is reached at.
    pub hosts: Vec<String>,
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
    backend: &Backend,
    prompt: &str,
    thinking: Option<&str>,
    start: Start,
) -> std::io::Result<Invocation> {
    let mut invocation = gateway_invocation(OPENCODE_HARNESS, route, backend)?;
    let url = gateway_url(OPENCODE_HARNESS, backend)?;
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

    // The run command reads the model from the configuration and takes no
    // model argument, so the gateway arguments are replaced.
    invocation.arguments = OPENCODE_RUN
        .iter()
        .map(|argument| argument.to_string())
        .collect();
    if start == Start::Resume {
        invocation.arguments.push(OPENCODE_CONTINUE.to_string());
    }
    invocation.arguments.push(prompt.to_string());

    invocation.arguments.push(OPENCODE_LOGS.to_string());
    invocation
        .files
        .push((OPENCODE_CONFIG_FILE.to_string(), configuration));

    Ok(invocation)
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
    backend: &Backend,
    prompt: &str,
    thinking: Option<&str>,
    start: Start,
) -> std::io::Result<Invocation> {
    let url = gateway_url(PI_HARNESS, backend)?;
    let models = template(
        PI_MODELS_TEMPLATE,
        &[
            ("__AVA_PROVIDER__", PI_PROVIDER),
            ("__AVA_BASE_URL__", url.as_str()),
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
    if start == Start::Resume {
        arguments.push(PI_CONTINUE.to_string());
    }
    arguments.push(prompt.to_string());

    Ok(Invocation {
        variables: vec![(GATEWAY_TOKEN.to_string(), backend.credential()?)],
        arguments,
        files: vec![(PI_MODELS_FILE.to_string(), models)],
        hosts: Vec::new(),
    })
}

/// Point codex at the gateway.
///
/// Codex resumes its recorded session through `exec resume`. The reasoning
/// effort travels in the configuration; codex has no per turn output knob, so
/// the turn cap goes unenforced here.
fn codex_invocation(
    route: &Route,
    backend: &Backend,
    prompt: &str,
    thinking: Option<&str>,
    start: Start,
) -> std::io::Result<Invocation> {
    let url = gateway_url(CODEX_HARNESS, backend)?;
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
        variables: vec![(GATEWAY_TOKEN.to_string(), backend.credential()?)],
        arguments: match start {
            Start::Task => CODEX_EXEC
                .iter()
                .map(|argument| argument.to_string())
                .chain([prompt.to_string()])
                .collect(),
            Start::Resume => CODEX_RESUME
                .iter()
                .map(|argument| argument.to_string())
                .chain([prompt.to_string()])
                .collect(),
        },
        files: vec![(CODEX_CONFIG_FILE.to_string(), configuration)],
        hosts: Vec::new(),
    })
}

/// The endpoint of `backend` as the gateway a third party harness is pointed
/// at.
///
/// The third party harnesses take the Anthropic provider slot and accept an
/// unknown model id under it, which is how a gateway model reaches them.
fn gateway_url(harness: &str, backend: &Backend) -> std::io::Result<String> {
    if backend.service != Service::OpenApi {
        return Err(std::io::Error::other(format!(
            "the {harness} harness reaches models only through an openapi gateway"
        )));
    }

    Ok(backend.url())
}

fn gateway_invocation(
    harness: &str,
    route: &Route,
    backend: &Backend,
) -> std::io::Result<Invocation> {
    let url = gateway_url(harness, backend)?;

    Ok(Invocation {
        variables: vec![
            (BASE_URL.to_string(), url),
            (GATEWAY_TOKEN.to_string(), backend.credential()?),
        ],
        arguments: vec![
            MODEL_OPTION.to_string(),
            format!("{GATEWAY_PROVIDER}/{}", route.id),
        ],
        files: Vec::new(),
        hosts: Vec::new(),
    })
}

fn claude_invocation(
    route: &Route,
    backend: &Backend,
    prompt: &str,
    thinking: Option<&str>,
    start: Start,
) -> std::io::Result<Invocation> {
    let mut environment: Vec<(String, String)> = CLAUDE_SETTINGS
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();

    environment.push((BASE_URL.to_string(), backend.url()));

    match backend.service {
        Service::Anthropic => {
            environment.push((CLAUDE_MODEL.to_string(), route.id.clone()));
            environment.push((SUBSCRIPTION_TOKEN.to_string(), backend.credential()?));
        }
        Service::OpenApi => {
            for name in CLAUDE_TIER_SETTINGS {
                environment.push((name.to_string(), route.id.clone()));
            }
            environment.push((GATEWAY_TOKEN.to_string(), backend.credential()?));
        }
    }

    let mut arguments = Vec::new();
    think(&mut arguments, CLAUDE_EFFORT, thinking);
    arguments.extend(CLAUDE_PRINT.iter().map(|argument| argument.to_string()));
    if start == Start::Resume {
        arguments.push(CLAUDE_CONTINUE.to_string());
    }
    arguments.push(prompt.to_string());

    Ok(Invocation {
        variables: environment,
        arguments,
        files: Vec::new(),
        hosts: Vec::new(),
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
pub(crate) fn load_or_exit() -> Registry {
    load().unwrap_or_else(|error| {
        eprintln!("error: {error}");
        std::process::exit(1);
    })
}

/// Print every known model with the backends serving it, then exit.
pub fn list_models() -> ! {
    let registry = load_or_exit();
    let names = registry.models.iter().map(|model| model.name.as_str());
    let width = column_width(MODEL_HEADER, names);

    println!("{MODEL_HEADER:<width$}  {BACKENDS_HEADER}");
    for model in &registry.models {
        let backends: Vec<&str> = model
            .routes
            .iter()
            .map(|route| route.backend.as_str())
            .collect();
        println!("{:<width$}  {}", model.name, backends.join(", "));
    }
    std::process::exit(0);
}

/// Print every known agent with the services it speaks, then exit.
pub fn list_agents() -> ! {
    let registry = load_or_exit();
    let names = registry.harnesses.iter().map(|agent| agent.name.as_str());
    let width = column_width(AGENT_HEADER, names);

    println!("{AGENT_HEADER:<width$}  {SERVICES_HEADER}");
    for agent in &registry.harnesses {
        let services: Vec<&str> = agent
            .services
            .iter()
            .map(|service| service.name())
            .collect();
        println!("{:<width$}  {}", agent.name, services.join(", "));
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
