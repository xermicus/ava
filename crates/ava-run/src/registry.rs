//! The backends, models and harnesses a benchmark run can pair into an agent,
//! and the agents named ahead.

const REGISTRY_FILE: &str = "registry.json";

/// The agents named ahead, a list beside the registry, which the agents page writes.
const AGENTS_FILE: &str = "agents.json";

/// Where a changed list is written before it replaces the file in one step.
const AGENTS_STAGING_FILE: &str = "agents.json.tmp";

/// Serializes every change to the agents file.
static AGENTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The characters an agent name is made of, besides letters and digits.
const NAME_PUNCTUATION: [char; 3] = ['-', '_', '.'];

/// The kind marking a registry that does not hold together, as opposed to a
/// file that cannot be read or written.
const INVALID: std::io::ErrorKind = std::io::ErrorKind::InvalidInput;

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
/// Codex refuses a workspace that is not a git repository, which the analysis workspace is not.
const CODEX_GIT_CHECK: &str = "--skip-git-repo-check";
const CODEX_EXEC: [&str; 3] = ["exec", CODEX_GIT_CHECK, "--json"];
const CODEX_RESUME: [&str; 5] = ["exec", "resume", "--last", CODEX_GIT_CHECK, "--json"];

/// The arguments printing every event of an unattended opencode run as a JSON line.
const OPENCODE_RUN: [&str; 4] = ["run", "--auto", "--format", "json"];

/// How much thinking a run asks for, weakest first.
///
/// These are the levels every harness expresses. Thinking is never turned off,
/// a benchmark measures the model as it is meant to be used.
pub const THINKING_LEVELS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// What separates the harness from the model when a pairing the registry does
/// not name is spelled out, `harness/model`.
pub const PAIRING_SEPARATOR: char = '/';

/// What separates the model from the backend when a route is spelled out,
/// `model@backend`.
pub const ROUTE_SEPARATOR: char = '@';

const CLAUDE_EFFORT: &str = "--effort";
const PI_THINKING: &str = "--thinking";

/// The arguments printing every event of an unattended claude run as a JSON
/// line, without which a run leaves no live log.
const CLAUDE_PRINT: [&str; 4] = ["--print", "--verbose", "--output-format", "stream-json"];

/// The output tokens claude code 2.1.247 asks for per request: the default of
/// its model catalog, or the fallback for a model the catalog does not know.
const CATALOG_TURN_OUTPUT: u32 = 64_000;
const FALLBACK_TURN_OUTPUT: u32 = 32_000;

/// The models with the larger catalog default.
const CATALOG_MODELS: [&str; 7] = [
    "claude-opus-4-6",
    "claude-opus-4-7",
    "claude-opus-4-8",
    "claude-opus-5",
    "claude-sonnet-5",
    "claude-fable-5",
    "claude-mythos-5",
];

/// The `max_output` of a route capped to what claude code sends for the model,
/// matched by its registry name or the last segment of its route id.
fn turn_output(model: &Model, route: &Route) -> u32 {
    let gateway_id = route.id.rsplit('/').next().unwrap_or(&route.id);
    let cap =
        if CATALOG_MODELS.contains(&model.name.as_str()) || CATALOG_MODELS.contains(&gateway_id) {
            CATALOG_TURN_OUTPUT
        } else {
            FALLBACK_TURN_OUTPUT
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

/// The variables pinning the window claude compacts within to the route; both only lower it.
const CLAUDE_CONTEXT_SETTINGS: [&str; 2] = [
    "CLAUDE_CODE_MAX_CONTEXT_TOKENS",
    "CLAUDE_CODE_AUTO_COMPACT_WINDOW",
];

/// Keeps claude from suffixing `[1m]` to a gateway model id, which would win over the route.
const CLAUDE_GATEWAY_CONTEXT: (&str, &str) = ("CLAUDE_CODE_DISABLE_1M_CONTEXT", "1");

/// What each harness prints once per compaction of its session.
const COMPACTION_MARKERS: [(&str, &str); 4] = [
    (CLAUDE_HARNESS, "\"subtype\":\"compact_boundary\""),
    (PI_HARNESS, "\"type\":\"compaction_end\""),
    (OPENCODE_HARNESS, "\"compaction_continue\":true"),
    (CODEX_HARNESS, "Long threads and multiple compactions"),
];

/// The text `harness` prints once per compaction of its session.
pub fn compaction_marker(harness: &str) -> Option<&'static str> {
    COMPACTION_MARKERS
        .iter()
        .find(|(name, _)| *name == harness)
        .map(|(_, marker)| *marker)
}

const CLAUDE_TIER_SETTINGS: [&str; 3] = [
    "ANTHROPIC_DEFAULT_OPUS_MODEL",
    "ANTHROPIC_DEFAULT_SONNET_MODEL",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
];

const MODEL_HEADER: &str = "MODEL";
const AGENT_HEADER: &str = "AGENT";
const HARNESS_HEADER: &str = "HARNESS";
const BACKEND_HEADER: &str = "BACKEND";
const ANALYST_HEADER: &str = "ANALYST";
const ANALYST_MARK: &str = "*";
const BACKENDS_HEADER: &str = "BACKENDS";
const SERVICES_HEADER: &str = "SERVICES";
const SERVICE_HEADER: &str = "SERVICE";
const HOST_HEADER: &str = "HOST";
const KEY_HEADER: &str = "KEY";

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

/// An agent named ahead: a harness paired with a model under a name of its
/// own, served by a backend.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Alias {
    pub name: String,
    pub harness: String,
    pub model: String,
    /// The backend serving the model, the first route of the model unless named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    /// Whether this is the agent analyzing runs unless another is chosen. One
    /// agent at most.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub analyst: bool,
}

impl Alias {
    /// The agent the alias names.
    pub fn agent(&self) -> ava_wire::Agent {
        ava_wire::Agent {
            harness: self.harness.clone(),
            model: self.model.clone(),
        }
    }
}

/// Every backend, model and harness a benchmark run may pair, and the agents
/// named ahead.
#[derive(serde::Deserialize)]
pub struct Registry {
    /// The backends the routes of the models name.
    pub backends: Vec<Backend>,
    /// The models a run may use.
    pub models: Vec<Model>,
    /// The harnesses a run may use.
    pub harnesses: Vec<Harness>,
    /// The agents named ahead, selectable by name, from the agents file.
    #[serde(skip)]
    pub agents: Vec<Alias>,
}

impl Registry {
    /// Resolve `setup` into the invocation running it.
    ///
    /// Credentials are read from the environment of this process rather than
    /// stored, so nothing secret reaches an image layer or the repository.
    ///
    /// Every turn of a run is one invocation: the first opens the session on
    /// `prompt` and every later one resumes the recorded session on it.
    pub fn invocation(
        &self,
        setup: &ava_wire::Setup,
        prompt: &str,
        start: Start,
    ) -> std::io::Result<Invocation> {
        let harness = self.harness(&setup.agent.harness)?;
        let (route, backend) = self.route(
            &setup.agent.harness,
            &setup.agent.model,
            setup.backend.as_deref(),
        )?;
        let thinking = setup.thinking.as_deref();

        log::info!(
            "{} reaches {} as {} on the {} backend",
            harness.name,
            setup.agent.model,
            route.id,
            backend.name
        );

        let output = turn_output(self.model(&setup.agent.model)?, route);
        let mut invocation = match harness.name.as_str() {
            CLAUDE_HARNESS => claude_invocation(route, backend, prompt, thinking, start),
            PI_HARNESS => pi_invocation(route, backend, prompt, thinking, output, start),
            OPENCODE_HARNESS => opencode_invocation(route, backend, prompt, output, start),
            CODEX_HARNESS => codex_invocation(route, backend, prompt, thinking, start),
            name => Err(std::io::Error::other(format!(
                "no adapter is defined for the {name} harness"
            ))),
        }?;

        invocation.hosts = self.hosts();
        invocation.context_window = route.context_window;
        invocation.backend = backend.name.clone();
        invocation.route = route.id.clone();

        Ok(invocation)
    }

    /// The route `harness` reaches `model` by: the one on `backend` when
    /// chosen, else the first route of the model on a service the harness
    /// speaks, walking the services in order, so a harness that can reach a
    /// model directly is not sent through a gateway.
    pub fn route(
        &self,
        harness: &str,
        model: &str,
        backend: Option<&str>,
    ) -> std::io::Result<(&Route, &Backend)> {
        let harness = self.harness(harness)?;
        let model = self.model(model)?;

        let mut served: Vec<(&Route, &Backend)> = Vec::new();
        for route in &model.routes {
            served.push((route, self.backend(&route.backend)?));
        }

        let chosen = match backend {
            Some(chosen) => {
                let chosen = self.backend(chosen)?;
                served
                    .iter()
                    .find(|(_, backend)| backend.name == chosen.name)
                    .filter(|(_, backend)| harness.services.contains(&backend.service))
                    .ok_or_else(|| {
                        std::io::Error::other(format!(
                            "the {} harness cannot serve {} from the {} backend",
                            harness.name, model.name, chosen.name
                        ))
                    })?
            }
            None => harness
                .services
                .iter()
                .find_map(|service| {
                    served
                        .iter()
                        .find(|(_, backend)| backend.service == *service)
                })
                .ok_or_else(|| {
                    std::io::Error::other(format!(
                        "the {} harness cannot serve {}",
                        harness.name, model.name
                    ))
                })?,
        };

        Ok(*chosen)
    }

    /// The setup `name` plays: an agent of the registry on its backend, or a
    /// harness paired with `model`, given beside it or after a slash, on the
    /// first route of the model, at `thinking`. Checked: the level is known
    /// and the harness serves the model there.
    pub fn setup(
        &self,
        name: &str,
        model: Option<&str>,
        thinking: Option<&str>,
    ) -> std::io::Result<ava_wire::Setup> {
        if let Some(level) = thinking
            && !THINKING_LEVELS.contains(&level)
        {
            return Err(unknown(
                level,
                "thinking level",
                THINKING_LEVELS.iter().copied(),
            ));
        }

        let agent = match (name.split_once(PAIRING_SEPARATOR), model) {
            (Some((harness, model)), None) => self.agent(harness, Some(model))?,
            _ => self.agent(name, model)?,
        };
        let backend = self
            .alias(name)
            .filter(|_| model.is_none())
            .and_then(|alias| alias.backend.as_deref());
        let (_, backend) = self.route(&agent.harness, &agent.model, backend)?;

        Ok(ava_wire::Setup {
            backend: Some(backend.name.clone()),
            agent,
            thinking: thinking.map(str::to_string),
        })
    }

    /// The agent `name` refers to: an alias, or a harness paired with `model`.
    pub fn agent(&self, name: &str, model: Option<&str>) -> std::io::Result<ava_wire::Agent> {
        match (self.alias(name), model) {
            (Some(alias), None) => Ok(alias.agent()),
            (Some(alias), Some(_)) => Err(std::io::Error::other(format!(
                "{} is an agent of the registry, its model is {}",
                alias.name, alias.model
            ))),
            (None, Some(model)) => {
                self.model(model)?;
                Ok(ava_wire::Agent {
                    harness: self.harness(name)?.name.clone(),
                    model: model.to_string(),
                })
            }
            (None, None) => Err(unknown(
                name,
                "agent",
                self.agents.iter().map(|alias| alias.name.as_str()),
            )),
        }
    }

    /// The alias named `name`.
    pub fn alias(&self, name: &str) -> Option<&Alias> {
        self.agents.iter().find(|alias| alias.name == name)
    }

    /// The agent analyzing runs unless another is chosen: the one marked as
    /// the analyst, else the first named.
    pub fn analyst(&self) -> Option<&Alias> {
        self.agents
            .iter()
            .find(|alias| alias.analyst)
            .or(self.agents.first())
    }

    /// The alias naming `agent`.
    pub fn alias_of(&self, agent: &ava_wire::Agent) -> Option<&Alias> {
        self.agents
            .iter()
            .find(|alias| alias.harness == agent.harness && alias.model == agent.model)
    }

    /// The model registered under `name`.
    pub fn model(&self, name: &str) -> std::io::Result<&Model> {
        self.models
            .iter()
            .find(|candidate| candidate.name == name)
            .ok_or_else(|| {
                unknown(
                    name,
                    "model",
                    self.models.iter().map(|entry| entry.name.as_str()),
                )
            })
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
    pub fn backend(&self, name: &str) -> std::io::Result<&Backend> {
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

/// Load the registry from `registry.json` in the working directory, with the
/// agents of `agents.json` beside it, none when there is no such file.
pub fn load() -> std::io::Result<Registry> {
    let registry = std::fs::read_to_string(REGISTRY_FILE)
        .map_err(|error| std::io::Error::other(format!("{REGISTRY_FILE}: {error}")))?;
    let agents = match std::fs::read_to_string(AGENTS_FILE) {
        Ok(agents) => agents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => NO_AGENTS.to_string(),
        Err(error) => {
            return Err(std::io::Error::other(format!("{AGENTS_FILE}: {error}")));
        }
    };

    parse(&registry, &agents)
        .map_err(|error| std::io::Error::other(format!("{REGISTRY_FILE}: {error}")))
}

/// What the agents file holds when there is none.
const NO_AGENTS: &str = "[]";

/// The registry `contents` hold with the `agents` listed beside it, checked.
fn parse(contents: &str, agents: &str) -> std::io::Result<Registry> {
    let mut registry: Registry = serde_json::from_str(contents).map_err(std::io::Error::other)?;
    registry.agents = serde_json::from_str(agents)
        .map_err(|error| std::io::Error::other(format!("{AGENTS_FILE}: {error}")))?;
    check(&registry)?;

    Ok(registry)
}

/// Whether `registry` holds together: every route names a backend, every
/// agent a harness that serves its model under a plain name no other agent or
/// harness has.
fn check(registry: &Registry) -> std::io::Result<()> {
    let invalid = |subject: &str, error: std::io::Error| {
        std::io::Error::new(INVALID, format!("{subject}: {error}"))
    };

    for model in &registry.models {
        for route in &model.routes {
            registry
                .backend(&route.backend)
                .map_err(|error| invalid(&model.name, error))?;
        }
    }

    for (index, alias) in registry.agents.iter().enumerate() {
        checked_name(&alias.name)?;
        let taken = registry.agents[..index]
            .iter()
            .any(|other| other.name == alias.name)
            || registry.harness(&alias.name).is_ok();
        if taken {
            return Err(std::io::Error::new(
                INVALID,
                format!("{}: the agent name is taken", alias.name),
            ));
        }
        registry
            .route(&alias.harness, &alias.model, alias.backend.as_deref())
            .map_err(|error| invalid(&alias.name, error))?;
    }

    if registry.agents.iter().filter(|alias| alias.analyst).count() > 1 {
        return Err(std::io::Error::new(
            INVALID,
            "one agent at most is the analyst",
        ));
    }

    Ok(())
}

/// Refuse an agent name that is not letters, digits, dashes, underscores and dots.
pub fn checked_name(name: &str) -> std::io::Result<()> {
    let plain = !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || NAME_PUNCTUATION.contains(&character)
        });

    if !plain {
        return Err(std::io::Error::new(
            INVALID,
            format!("`{name}`: an agent name is letters, digits, dashes, underscores and dots"),
        ));
    }

    Ok(())
}

/// Whether `error` reports a registry that does not hold together, which the
/// change that made it so is to correct.
pub fn is_invalid(error: &std::io::Error) -> bool {
    error.kind() == INVALID
}

/// Change the agents file under the agents lock: loaded with the registry,
/// changed, checked against it, written whole.
fn modify(change: impl FnOnce(&mut Registry) -> std::io::Result<()>) -> std::io::Result<()> {
    let _agents = AGENTS.lock().expect("the agents lock is not poisoned");
    let mut registry = load()?;
    change(&mut registry)?;
    check(&registry)?;

    let contents = serde_json::to_string_pretty(&registry.agents).map_err(std::io::Error::other)?;
    std::fs::write(AGENTS_STAGING_FILE, format!("{contents}\n"))?;
    std::fs::rename(AGENTS_STAGING_FILE, AGENTS_FILE)
}

/// Name `alias` in the agents file.
pub fn add_alias(alias: Alias) -> std::io::Result<()> {
    modify(|registry| {
        if alias.analyst {
            demote_analyst(registry);
        }
        registry.agents.push(alias);
        Ok(())
    })
}

/// Unmark the analyst, for another agent to take the mark.
fn demote_analyst(registry: &mut Registry) {
    for alias in &mut registry.agents {
        alias.analyst = false;
    }
}

/// Replace the agent named `name` in the agents file with `alias`.
pub fn replace_alias(name: &str, alias: Alias) -> std::io::Result<()> {
    modify(|registry| {
        if alias.analyst {
            demote_analyst(registry);
        }
        let replaced = registry
            .agents
            .iter_mut()
            .find(|known| known.name == name)
            .ok_or_else(|| std::io::Error::new(INVALID, format!("no agent named {name}")))?;
        *replaced = alias;
        Ok(())
    })
}

/// Remove the agent named `name` from the agents file.
pub fn remove_alias(name: &str) -> std::io::Result<()> {
    modify(|registry| {
        let before = registry.agents.len();
        registry.agents.retain(|alias| alias.name != name);
        if registry.agents.len() == before {
            return Err(std::io::Error::new(
                INVALID,
                format!("no agent named {name}"),
            ));
        }
        Ok(())
    })
}

/// How a harness is told which model to use and how to authenticate.
#[derive(Clone, Default)]
pub struct Invocation {
    /// The environment handed to the container.
    pub variables: Vec<(String, String)>,
    /// The arguments appended to the image entrypoint.
    pub arguments: Vec<String>,
    /// Configuration written into the container, by path and contents.
    pub files: Vec<(String, String)>,
    /// The context window of the route.
    pub context_window: u32,
    /// The hosts the sandbox resolves to loopback, where the bridge forwards
    /// them onto the proxy: every host a registered backend is reached at.
    pub hosts: Vec<String>,
    /// The backend the model is served by and the id it is asked for there.
    pub backend: String,
    pub route: String,
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
    output: u32,
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
            ("\"__AVA_OUTPUT__\"", output.to_string().as_str()),
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
    output: u32,
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
            ("\"__AVA_OUTPUT__\"", output.to_string().as_str()),
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
    for name in CLAUDE_CONTEXT_SETTINGS {
        environment.push((name.to_string(), route.context_window.to_string()));
    }

    match backend.service {
        Service::Anthropic => {
            environment.push((CLAUDE_MODEL.to_string(), route.id.clone()));
            environment.push((SUBSCRIPTION_TOKEN.to_string(), backend.credential()?));
        }
        Service::OpenApi => {
            for name in CLAUDE_TIER_SETTINGS {
                environment.push((name.to_string(), route.id.clone()));
            }
            let (name, value) = CLAUDE_GATEWAY_CONTEXT;
            environment.push((name.to_string(), value.to_string()));
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
        ..Default::default()
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

/// Print every agent named in the registry, then exit.
pub fn list_agents() -> ! {
    let registry = load_or_exit();
    let names = registry.agents.iter().map(|alias| alias.name.as_str());
    let width = column_width(AGENT_HEADER, names);
    let harnesses = registry.agents.iter().map(|alias| alias.harness.as_str());
    let harness_width = column_width(HARNESS_HEADER, harnesses);
    let models = registry.agents.iter().map(|alias| alias.model.as_str());
    let model_width = column_width(MODEL_HEADER, models);

    let backends = registry
        .agents
        .iter()
        .map(|alias| alias.backend.as_deref().unwrap_or_default());
    let backend_width = column_width(BACKEND_HEADER, backends);

    println!(
        "{AGENT_HEADER:<width$}  {HARNESS_HEADER:<harness_width$}  {MODEL_HEADER:<model_width$}  {BACKEND_HEADER:<backend_width$}  {ANALYST_HEADER}"
    );
    for alias in &registry.agents {
        println!(
            "{:<width$}  {:<harness_width$}  {:<model_width$}  {:<backend_width$}  {}",
            alias.name,
            alias.harness,
            alias.model,
            alias.backend.as_deref().unwrap_or_default(),
            if alias.analyst { ANALYST_MARK } else { "" }
        );
    }
    std::process::exit(0);
}

/// Print every known backend with its service, host and the variable holding
/// its key, then exit.
pub fn list_backends() -> ! {
    let registry = load_or_exit();
    let width = column_width(
        BACKEND_HEADER,
        registry
            .backends
            .iter()
            .map(|backend| backend.name.as_str()),
    );
    let service_width = column_width(
        SERVICE_HEADER,
        registry
            .backends
            .iter()
            .map(|backend| backend.service.name()),
    );
    let host_width = column_width(
        HOST_HEADER,
        registry
            .backends
            .iter()
            .map(|backend| backend.host.as_str()),
    );

    println!(
        "{BACKEND_HEADER:<width$}  {SERVICE_HEADER:<service_width$}  {HOST_HEADER:<host_width$}  {KEY_HEADER}"
    );
    for backend in &registry.backends {
        println!(
            "{:<width$}  {:<service_width$}  {:<host_width$}  {}",
            backend.name,
            backend.service.name(),
            backend.host,
            backend.key
        );
    }
    std::process::exit(0);
}

/// Print every known harness with the services it speaks, then exit.
pub fn list_harnesses() -> ! {
    let registry = load_or_exit();
    let names = registry
        .harnesses
        .iter()
        .map(|harness| harness.name.as_str());
    let width = column_width(HARNESS_HEADER, names);

    println!("{HARNESS_HEADER:<width$}  {SERVICES_HEADER}");
    for harness in &registry.harnesses {
        let services: Vec<&str> = harness
            .services
            .iter()
            .map(|service| service.name())
            .collect();
        println!("{:<width$}  {}", harness.name, services.join(", "));
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

#[cfg(test)]
mod tests {
    const REGISTRY: &str = r#"{
        "backends": [
            {"name": "direct", "service": "anthropic", "host": "a.example", "key": "A"},
            {"name": "gateway", "service": "openapi", "host": "g.example", "key": "G"},
            {"name": "other", "service": "openapi", "host": "o.example", "key": "O"}
        ],
        "models": [
            {"name": "m", "routes": [
                {"backend": "direct", "id": "m-direct", "context_window": 1, "max_output": 1},
                {"backend": "gateway", "id": "m-gateway", "context_window": 1, "max_output": 1},
                {"backend": "other", "id": "m-other", "context_window": 1, "max_output": 1}
            ]}
        ],
        "harnesses": [
            {"name": "claude", "services": ["anthropic", "openapi"]},
            {"name": "pi", "services": ["openapi"]}
        ]
    }"#;
    const AGENTS: &str = r#"[
        {"name": "opus", "harness": "pi", "model": "m", "backend": "other"},
        {"name": "judge", "harness": "claude", "model": "m", "analyst": true}
    ]"#;

    #[test]
    fn the_route_is_the_first_the_harness_speaks_unless_a_backend_is_chosen() {
        let registry = super::parse(REGISTRY, AGENTS).unwrap();

        assert_eq!(
            registry.route("claude", "m", None).unwrap().1.name,
            "direct"
        );
        assert_eq!(registry.route("pi", "m", None).unwrap().1.name, "gateway");
        assert_eq!(
            registry.route("pi", "m", Some("other")).unwrap().0.id,
            "m-other"
        );
        assert!(registry.route("pi", "m", Some("direct")).is_err());
    }

    #[test]
    fn an_alias_resolves_alone_and_a_harness_with_its_model() {
        let registry = super::parse(REGISTRY, AGENTS).unwrap();

        assert_eq!(registry.agent("opus", None).unwrap().label(), "pi on m");
        assert_eq!(registry.agent("pi", Some("m")).unwrap().label(), "pi on m");
        assert!(registry.agent("opus", Some("m")).is_err());
        assert!(registry.agent("pi", None).is_err());
        assert!(registry.agent("nobody", Some("m")).is_err());
    }

    #[test]
    fn a_setup_takes_a_name_or_a_pairing_and_refuses_what_cannot_play() {
        let registry = super::parse(REGISTRY, AGENTS).unwrap();

        let named = registry.setup("opus", None, Some("low")).unwrap();
        assert_eq!(named.label(), "pi on m at low via other");
        let spelled = registry.setup("claude/m", None, None).unwrap();
        assert_eq!(spelled.label(), "claude on m via direct");
        assert!(registry.setup("opus", None, Some("lots")).is_err());
        assert!(registry.setup("nobody", None, None).is_err());

        let unreachable = AGENTS.replace(r#""backend": "other""#, r#""backend": "direct""#);
        assert!(super::parse(REGISTRY, &unreachable).is_err());
        let unknown = AGENTS.replace(r#""backend": "other""#, r#""backend": "nowhere""#);
        assert!(super::parse(REGISTRY, &unknown).is_err());
    }

    #[test]
    fn one_agent_at_most_is_the_analyst() {
        let registry = super::parse(REGISTRY, AGENTS).unwrap();
        assert_eq!(registry.analyst().unwrap().name, "judge");

        let two = AGENTS.replace(
            r#""backend": "other""#,
            r#""backend": "other", "analyst": true"#,
        );
        assert!(super::parse(REGISTRY, &two).is_err());

        let none = AGENTS.replace(r#", "analyst": true"#, "");
        let registry = super::parse(REGISTRY, &none).unwrap();
        assert_eq!(registry.analyst().unwrap().name, "opus");
    }

    #[test]
    fn an_agent_name_is_not_a_harness_name_and_names_a_pairing_that_plays() {
        let taken = AGENTS.replace(r#""name": "opus""#, r#""name": "pi""#);
        assert!(super::parse(REGISTRY, &taken).is_err());

        let unserved = AGENTS.replace(r#""model": "m""#, r#""model": "x""#);
        assert!(super::parse(REGISTRY, &unserved).is_err());

        let spaced = AGENTS.replace(r#""name": "opus""#, r#""name": "op us""#);
        let Err(error) = super::parse(REGISTRY, &spaced) else {
            panic!("a spaced name parses");
        };
        assert!(super::is_invalid(&error));
    }

    #[test]
    fn the_agents_write_back_the_way_they_read() {
        let registry = super::parse(REGISTRY, AGENTS).unwrap();
        let written = serde_json::to_string_pretty(&registry.agents).unwrap();
        let reread = super::parse(REGISTRY, &written).unwrap();

        assert_eq!(reread.agents, registry.agents);
        assert_eq!(
            written,
            serde_json::to_string_pretty(&reread.agents).unwrap()
        );
    }

    fn route(id: &str, max_output: u32) -> super::Route {
        super::Route {
            backend: "gateway".to_string(),
            id: id.to_string(),
            context_window: 1_000_000,
            max_output,
        }
    }

    fn model(name: &str) -> super::Model {
        super::Model {
            name: name.to_string(),
            routes: Vec::new(),
        }
    }

    #[test]
    fn a_turn_spends_what_claude_code_sends_for_the_model() {
        let direct = route("claude-opus-5", 128_000);
        let gateway = route("openrouter/anthropic/claude-opus-5", 128_000);
        let unknown = route("openrouter/z-ai/glm-5.3", 131_072);

        assert_eq!(super::turn_output(&model("claude-opus-5"), &direct), 64_000);
        assert_eq!(super::turn_output(&model("opus"), &gateway), 64_000);
        assert_eq!(super::turn_output(&model("glm-5.3"), &unknown), 32_000);
        assert_eq!(
            super::turn_output(
                &model("claude-haiku-4-5"),
                &route("claude-haiku-4-5", 64_000)
            ),
            32_000
        );
        assert_eq!(
            super::turn_output(&model("claude-sonnet-5"), &route("claude-sonnet-5", 16_000)),
            16_000
        );
    }
}
