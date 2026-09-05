//! Docker orchestration of the containers taking part in a benchmark run.

use crate::process;

/// The agent sandbox command.
#[derive(Debug, Clone)]
pub struct Agent {
    /// The agent to run, naming a directory under `agents`.
    pub name: String,
    /// The model the agent runs on.
    pub model: String,
    /// The game to play and score, naming a directory under `games`.
    pub game: String,
    /// The wall clock time the agent is given, in seconds, the last call
    /// included.
    pub limit: u64,
    /// How many runs are started in parallel.
    pub parallel: u64,
    /// How much thinking the agent is asked for.
    pub thinking: Option<String>,
    /// Whether the docker images are rebuilt instead of reused.
    pub force_build_images: bool,
    /// The agent analyzing the run once it is over.
    pub analyst: Option<Analyst>,
    /// The entry the run attacks, when it plays a pairing of a tournament.
    pub challenge: Option<Challenge>,
}

impl Agent {
    /// The seconds an agent is given unless the command names a limit.
    pub const DEFAULT_LIMIT_SECONDS: u64 = 300;
    /// The runs started unless the command names a count.
    pub const DEFAULT_PARALLEL_RUNS: u64 = 1;

    /// `limit` as the seconds a run may be given: it pays for the last call,
    /// so it is at least that.
    pub fn checked_limit(limit: u64) -> std::io::Result<u64> {
        if limit < LAST_CALL_SECONDS {
            return Err(std::io::Error::other(format!(
                "the seconds pay for the last call, so they are at least {LAST_CALL_SECONDS}"
            )));
        }

        Ok(limit)
    }

    /// The agent of the command.
    pub fn agent(&self) -> ava_wire::Agent {
        ava_wire::Agent {
            harness: self.name.clone(),
            model: self.model.clone(),
            thinking: self.thinking.clone(),
        }
    }
}

impl Default for Agent {
    fn default() -> Self {
        Self {
            name: String::new(),
            model: String::new(),
            game: String::new(),
            limit: Self::DEFAULT_LIMIT_SECONDS,
            parallel: Self::DEFAULT_PARALLEL_RUNS,
            thinking: None,
            force_build_images: false,
            analyst: None,
            challenge: None,
        }
    }
}

/// The entry a run attacks: the file on disk, and the run and attempt it came from.
#[derive(Debug, Clone)]
pub struct Challenge {
    pub path: std::path::PathBuf,
    pub record: ava_wire::Challenge,
}

/// The image building command.
#[derive(Debug, Default)]
pub struct Image {
    /// The harness whose image is built, naming a directory under `agents`.
    pub agent: String,
    /// Whether the proxy image is built.
    pub proxy: bool,
    /// Whether the scorer image is built.
    pub scorer: bool,
}

/// The agent analyzing a run.
#[derive(Debug, Clone)]
pub struct Analyst {
    /// The harness, naming a directory under `agents`.
    pub name: String,
    /// The model the harness runs on.
    pub model: String,
    /// How much thinking the analyst is asked for.
    pub thinking: Option<String>,
    /// The seconds it is given.
    pub limit: u64,
}

impl Analyst {
    /// The seconds an analyst is given unless one is chosen.
    pub const DEFAULT_LIMIT_SECONDS: u64 = ava_wire::DEFAULT_ANALYST_SECONDS;

    /// `limit` as the seconds an analyst may be given: at least a last call.
    pub fn checked_limit(limit: u64) -> std::io::Result<u64> {
        if limit < LAST_CALL_SECONDS {
            return Err(std::io::Error::other(format!(
                "the analyst gets at least {LAST_CALL_SECONDS} seconds"
            )));
        }

        Ok(limit)
    }

    /// The agent of the analyst.
    pub fn agent(&self) -> ava_wire::Agent {
        ava_wire::Agent {
            harness: self.name.clone(),
            model: self.model.clone(),
            thinking: self.thinking.clone(),
        }
    }
}

impl Default for Analyst {
    fn default() -> Self {
        Self {
            name: String::new(),
            model: String::new(),
            thinking: None,
            limit: Self::DEFAULT_LIMIT_SECONDS,
        }
    }
}

/// The run analysis command.
#[derive(Debug, Default)]
pub struct Analyze {
    /// The run to analyze, naming a directory under `runs`.
    pub run: String,
    /// The agent analyzing it, with the seconds it is given.
    pub analyst: Analyst,
}

const NETWORK_EGRESS: &str = "ava-egress";
const SOCKET_VOLUME_PREFIX: &str = "ava-sockets-";
const SOCKET_DIRECTORY: &str = "/run/ava";
const SOCKET_PATH: &str = "/run/ava/proxy.sock";
const SCORE_SOCKET_PATH: &str = "/run/ava/score.sock";
const PROXY_CONTAINER_PREFIX: &str = "ava-proxy-";
pub const SCORER_CONTAINER_PREFIX: &str = "ava-scorer-";
const SANDBOX_CONTAINER_PREFIX: &str = "ava-agent-";

/// The suffix naming the containers and volumes of an analysis.
const ANALYSIS_SUFFIX: &str = "-analysis";
const RUN_MOUNT: &str = "/home/agent/run";

/// The files the analysis writes into the run directory.
const ANALYSIS_PREFIX: &str = "analysis";

/// The report the analyst writes into its workspace.
const REPORT_FILE: &str = "analysis.json";
const BOOK_DIRECTORY: &str = "book/src";
const BOOK_MOUNT: &str = "/home/agent/ava-book";

/// The hosts the proxy routes onto the scoring socket.
const GIT_HOST: &str = "git";
const SCORER_HOST: &str = "score";
const SCORE_ENTRY: &str = "/home/agent/score-entry.sh";
const BASH: &str = "bash";
const ROOT_USER: &str = "0";
const PROXY_IMAGE: &str = "ava/openapi-proxy";
const PROXY_CONTEXT: &str = "openapi-proxy";
const PROXY_HOSTS: &str = "openapi-proxy/hosts.conf";
const CONTAINER_HOSTS: &str = "/etc/nginx/conf.d/hosts.conf";
const READ_ONLY: &str = ":ro";
const BASE_IMAGE: &str = "ava/base";
const BASE_CONTEXT: &str = "agents";

/// The build context of every game image, so a Dockerfile reaches the
/// material games share.
const GAME_CONTEXT: &str = "games";
const GAME_DOCKERFILE: &str = "Dockerfile";

/// The build argument naming the image a game Dockerfile layers on.
const BASE_ARGUMENT: &str = "BASE";
const AGENT_IMAGE_PREFIX: &str = "ava/agent-";
const AGENT_CONTEXT: &str = "agents";
const SANDBOX_WORKSPACE: &str = "/home/agent/workspace";
const SANDBOX_TMPFS: &str = "/tmp:exec";
const HOME_VOLUME_PREFIX: &str = "ava-home-";
const WORKSPACE_VOLUME_PREFIX: &str = "ava-work-";
const HOLDER_CONTAINER_PREFIX: &str = "ava-holder-";

/// The agent home, overmounted with the memory backed home of the run.
const AGENT_HOME: &str = "/home/agent";

/// Where the home volume is mounted while it is held open and seeded.
const HOME_STAGE: &str = "/seed";

/// Where the workspace volume is mounted for the same, nested in the home the
/// way it is nested in the sandbox.
const WORKSPACE_STAGE: &str = "/seed/workspace";

/// The size of the agent home.
///
/// A tmpfs enforces this in the kernel, which the container filesystem cannot
/// do without a host quota. It has to cover the image home copied into it on
/// top of everything the harness writes, which is where a runaway tool output
/// lands. Filling it costs the harness its session and nothing else, since the
/// workspace is a volume of its own.
const HOME_SIZE: &str = "size=4g";

/// The size of the workspace.
///
/// Its own tmpfs, so the harness cannot spend the room the submission needs
/// and the agent cannot spend the room the harness needs. This is the side a
/// full filesystem would cost the run its submission, since git has to write
/// to commit, so it is not sized to absorb anything beyond the task.
const WORKSPACE_SIZE: &str = "size=4g";

/// The bytes any one file in the sandbox may reach.
///
/// A runaway write dies of `SIGXFSZ` at this size instead of taking a whole
/// tmpfs with it. The limit is per file rather than a total, so it is half of
/// the smaller of the two volumes and stops one file short of filling it.
const MAX_FILE_BYTES: &str = "fsize=2147483648";

/// The size of the scratch space, which no restart carries over.
const SCRATCH_SIZE: &str = "size=2g";

/// The owner of everything under the agent home.
const SANDBOX_OWNER: &str = "1000:1000";

/// Holds the home volume mounted for the whole run and does nothing else.
const HOLD_OPEN: &str = "sleep infinity";

/// Without this, crashing submissions leave a core dump into the host journal.
const NO_CORE_DUMPS: &str = "core=0";

/// Longer than the scoring timeout of the server, so the drain probe outlives
/// any scoring still in flight without hanging teardown on a dead server.
const DRAIN_TIMEOUT_SECONDS: &str = "90";
const DRAIN_PROBE_URL: &str = "http://git/task.git/info/refs?service=git-upload-pack";
const SANDBOX_LOOPBACK: &str = "127.0.0.1";
const STAGING_DIRECTORY: &str = "ava-agent-config";
pub const RUN_DIRECTORY: &str = "runs";
pub const ACCESS_LOG: &str = "proxy.access.log";
pub const ERROR_LOG: &str = "proxy.error.log";
pub const SCORE_LOG: &str = "score.log";
pub const SCORE_ERROR_LOG: &str = "score.error.log";
pub const AGENT_LOG: &str = "agent.log";
pub const ANALYSIS_FILE: &str = "analysis.json";
pub const ANALYSIS_LOG: &str = "analysis.log";
pub const ANALYSIS_ACCESS_LOG: &str = "analysis.access.log";
pub const ANALYSIS_ERROR_LOG: &str = "analysis.error.log";
const RECORD_BUFFER_BYTES: usize = 8 * 1024;

/// The bytes `agent.log` may reach, ended by a line saying it was cut.
const AGENT_LOG_CEILING: u64 = 4 * 1024 * 1024 * 1024;
const AGENT_LOG_CUT: &str = "\n[ava] the console reached its ceiling, the rest is not recorded\n";

const KIBIBYTE: u64 = 1024;

/// How often the run loop reports the state of the run.
const STATUS_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// Silence long enough to warn about, and to warn about again while it lasts.
const SILENCE_WARNING: std::time::Duration = std::time::Duration::from_secs(120);

/// The marker the receive hook leaves once the agent pushes a release tag.
pub const DONE_MARKER: &str = "/home/agent/done";

/// The seconds the agent gets to answer the last call.
pub const LAST_CALL_SECONDS: u64 = 120;

/// The seconds the turn loop may spend of a `total` budget.
fn loop_seconds(total: u64) -> u64 {
    total.saturating_sub(LAST_CALL_SECONDS)
}

/// The seconds a turn ending faster than this waits before the next one, so a
/// harness that fails at startup cannot spin through the whole clock.
const TURN_RETRY_SECONDS: u64 = 5;

/// The branch a submission is pushed to.
const TASK_BRANCH: &str = "task";

/// The entrypoint every harness image starts through, which forwards the git
/// host onto the proxy socket.
const BRIDGE: &str = "ava-bridge";

/// The message on the commit ava makes on the agent's behalf.
const LAST_CHANCE_MESSAGE: &str = "last chance";

/// How long the last chance keeps trying to reach the git host, which the
/// bridge forwards from a socat it has only just started.
const PUSH_ATTEMPTS: u32 = 50;
const PUSH_INTERVAL_SECONDS: &str = "0.2";

/// The record of one run, written when the run starts and completed when it is over.
pub const RUN_FILE: &str = "run.json";
const VERSION_OPTION: &str = "--version";
const IMAGE_ID_FORMAT: &str = "{{.Id}}";
const ARCHITECTURE_FORMAT: &str = "{{.Architecture}}";

/// What marks the version of a game whose folder differs from its last commit.
const DIRTY_SUFFIX: &str = "-dirty";

const SCORER_IMAGE: &str = "ava/scorer";
const SCORER_DOCKERFILE: &str = "scorer/Dockerfile";
const REPOSITORY_CONTEXT: &str = ".";
const GAMES_DIRECTORY: &str = "games";
const TASK_DIRECTORY: &str = "task";
/// The tasks of a multiplayer game whose entries agents attack: defending an
/// entry, and attacking the entry of another seat.
const DEFEND_DIRECTORY: &str = "defend";
const ATTACK_DIRECTORY: &str = "attack";
const TASK_INSTRUCTIONS: &str = "README.md";

const TASK_MOUNT: &str = "/home/agent/task";
const README_MOUNT: &str = "/home/agent/README.md";

/// Where the scorer keeps the entry of every passing attempt, by the seconds
/// of the attempt, and the folder of the run they are collected into.
pub const ENTRIES_DIRECTORY: &str = "entries";
const SCORER_ENTRIES: &str = "/home/agent/entries";

/// Where the entry a run attacks is mounted into the scoring container, which
/// seeds it into the workspace and verifies the pushes against it.
const CHALLENGE_MOUNT: &str = "/home/agent/challenge";

/// Where the two entries of a fight are mounted into the scorer image.
const FIGHT_MOUNT: &str = "/home/agent/fight";
const FIGHT_STAGING_PREFIX: &str = "ava-fight-";

/// The seconds one fight may take, enforced by coreutils timeout in the scorer image.
const FIGHT_TIMEOUT_SECONDS: &str = "600";
const TIMEOUT: &str = "timeout";
const AVA: &str = "ava";
const SANDBOX_USER: &str = "1000";
const HOME_VARIABLE: &str = "HOME";

/// Tells apart the fights one process stages at the same time.
static FIGHT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The run loop clock as `runs/<run>/monitor.json`, freshened on every status
/// tick. The loop clock is monotonic and pauses with the host, so wall clock
/// arithmetic overstates a live run whenever the host slept.
pub const MONITOR_FILE: &str = "monitor.json";
const READY_ATTEMPTS: u32 = 100;
const READY_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
const CLOCK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// The prompt a harness is started on.
const TASK_PROMPT: &str = "Read README.md in your workspace and work on the task it lays out.";

/// The task an analyst is started on.
const ANALYSIS_PROMPT: &str = include_str!("../assets/analysis.md");

/// Counts the launches of this process, telling apart the runs a long lived
/// process such as the web interface starts one after another.
static RUN_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The unique name of one launch of `agent`, extending the process id with the
/// launch count once the first launch took the plain name.
pub fn run_name(agent: &str) -> String {
    let launch = RUN_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    if launch == 0 {
        format!("{agent}-{}", std::process::id())
    } else {
        format!("{agent}-{}r{launch}", std::process::id())
    }
}

/// Every run directory on disk, in no particular order.
///
/// A run directory that is not there holds no runs, which is what a fresh
/// checkout looks like until the first run creates it.
pub fn run_directories() -> std::io::Result<Vec<std::path::PathBuf>> {
    match std::fs::read_dir(RUN_DIRECTORY) {
        Ok(entries) => Ok(entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(std::io::Error::new(
            error.kind(),
            format!("{RUN_DIRECTORY}: {error}"),
        )),
    }
}

/// The proxy container serving one run.
pub fn proxy_container(run: &str) -> String {
    format!("{PROXY_CONTAINER_PREFIX}{run}")
}

/// The scoring container serving one run.
pub fn scorer_container(run: &str) -> String {
    format!("{SCORER_CONTAINER_PREFIX}{run}")
}

/// The sandbox container of one run.
pub fn sandbox_container(run: &str) -> String {
    format!("{SANDBOX_CONTAINER_PREFIX}{run}")
}

/// The name of the analysis of `run`.
fn analysis_name(run: &str) -> String {
    format!("{run}{ANALYSIS_SUFFIX}")
}

/// The holder container of the analysis of `run`.
pub fn analyst_container(run: &str) -> String {
    holder_container(&analysis_name(run))
}

/// The volume carrying the socket between one sandbox and its proxy.
fn socket_volume(run: &str) -> String {
    format!("{SOCKET_VOLUME_PREFIX}{run}")
}

/// The memory backed home of one run, outliving the sandbox that writes it.
fn home_volume(run: &str) -> String {
    format!("{HOME_VOLUME_PREFIX}{run}")
}

/// The volume carrying the workspace of one run, nested in its home.
fn workspace_volume(run: &str) -> String {
    format!("{WORKSPACE_VOLUME_PREFIX}{run}")
}

/// The volume arguments handing one run its home and its workspace.
///
/// Docker mounts by the depth of the destination, so the workspace lands on
/// the mount point inside the home whichever order these are passed in.
fn home_mounts(run: &str) -> [String; 4] {
    [
        String::from("--volume"),
        format!("{}:{AGENT_HOME}", home_volume(run)),
        String::from("--volume"),
        format!("{}:{SANDBOX_WORKSPACE}", workspace_volume(run)),
    ]
}

/// The container keeping the home of one run mounted between its two starts.
fn holder_container(run: &str) -> String {
    format!("{HOLDER_CONTAINER_PREFIX}{run}")
}

/// The prompt one turn of the loop is started on.
///
/// Every harness gets this same text, because the loop is `ava` starting the
/// harness again rather than anything a harness does to itself.
fn loop_prompt(turn: u32, task: &str) -> String {
    format!("Loop iteration {turn}.\n\n{task}")
}

/// The prompt the agent is restarted on once its time is up.
///
/// The bound is stated as the turn rather than the seconds it is given. This
/// start carries no loop, so it really is one turn, while a wall clock is
/// something the agent would have to guess its own latency against and nothing
/// meters what it spends.
fn last_call_prompt() -> String {
    String::from(
        "Time is up. This is your final turn. Submit right now: commit and push what you have. \
         Submitting is free: a push that fails the verifier costs nothing, and every push that \
         passed it keeps its entry.",
    )
}

/// Copy the image home into the home volume and hand it to the sandbox user.
fn seed_home_command() -> String {
    format!(
        "tar -C {AGENT_HOME} -cf - . | tar -C {HOME_STAGE} -xf - && chown -R {SANDBOX_OWNER} {HOME_STAGE}"
    )
}

/// Create the agent home of one run, keep it mounted and seed it.
///
/// The holder starts first and stays for the whole run: a tmpfs is torn down
/// once the last container using it exits, which would drop the session
/// between the two starts. Seeding comes after, because overmounting the home
/// hides what the image installed there and the volume is not populated from
/// the image on its own.
fn prepare_agent_home(run: &str, image: &str) -> std::io::Result<()> {
    let holder = holder_container(run);
    let home = format!("{}:{HOME_STAGE}", home_volume(run));
    let workspace = format!("{}:{WORKSPACE_STAGE}", workspace_volume(run));

    for (volume, size) in [
        (home_volume(run), HOME_SIZE),
        (workspace_volume(run), WORKSPACE_SIZE),
    ] {
        log::info!("creating {volume} with {size}");
        process::run_and_assume_success(
            "docker",
            &[
                "volume",
                "create",
                "--driver",
                "local",
                "--opt",
                "type=tmpfs",
                "--opt",
                "device=tmpfs",
                "--opt",
                &format!("o={size}"),
                &volume,
            ],
        )?;
    }

    process::run_and_assume_success(
        "docker",
        &[
            "run",
            "--detach",
            "--name",
            &holder,
            "--network",
            "none",
            "--read-only",
            "--volume",
            &home,
            "--volume",
            &workspace,
            "--entrypoint",
            BASH,
            image,
            "-c",
            HOLD_OPEN,
        ],
    )?;

    log::info!("seeding the home of {run} from {image}");
    process::run_and_assume_success(
        "docker",
        &[
            "run",
            "--rm",
            "--network",
            "none",
            "--user",
            ROOT_USER,
            "--volume",
            &home,
            "--volume",
            &workspace,
            "--entrypoint",
            BASH,
            image,
            "-c",
            &seed_home_command(),
        ],
    )?;

    Ok(())
}

/// Commit and push the workspace on the agent's behalf.
///
/// One string, because it runs as the argument of a shell in the harness
/// image. A commit that finds nothing to add fails, which is no reason to skip
/// the push, since there may be commits the agent never pushed.
///
/// The push is forced, because a task branch the agent left ahead of its own
/// working tree would reject a plain one and the submission would be lost for
/// nothing: attempts and their entries are recorded as they arrive, nothing
/// reads the branch afterwards, and the repository goes with the scorer at
/// teardown.
fn last_chance_command() -> String {
    format!(
        "echo \"branch=$(git branch --show-current) head=$(git rev-parse --short HEAD)\"; \
         git add --all; \
         git commit --quiet --message '{LAST_CHANCE_MESSAGE}' || echo 'nothing new to commit'; \
         for _ in $(seq {PUSH_ATTEMPTS}); do \
             git push --force --quiet origin HEAD:refs/heads/{TASK_BRANCH} && exit 0; \
             sleep {PUSH_INTERVAL_SECONDS}; \
         done; \
         echo 'the git host never answered'; \
         exit 0"
    )
}

/// Submit whatever the agent left behind, once it has had its last call.
///
/// An agent can spend its last call reasoning about an optimisation and never
/// commit the working one it already had. Every passing attempt keeps its
/// entry and the entry of record is picked among them, so a push worse than
/// one already graded changes nothing, and a broken one is only a failed
/// attempt. The single thing this can do is recover work that would otherwise
/// go with the home.
///
/// The sandbox is gone by now, so this runs in a container of its own on the
/// same home, entered through the bridge so the git host resolves and the
/// proxy socket carries the push. The sidecars are still up, since `play`
/// removes them only once the run is over.
fn last_chance(run: &str, image: &str) {
    log::info!("{run}: submitting what the agent left, on its behalf");

    let output = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "none",
            "--read-only",
            "--tmpfs",
            &format!("{SANDBOX_TMPFS},{SCRATCH_SIZE}"),
            "--volume",
            &format!("{}:{SOCKET_DIRECTORY}{READ_ONLY}", socket_volume(run)),
            "--add-host",
            &format!("{GIT_HOST}:{SANDBOX_LOOPBACK}"),
            "--workdir",
            SANDBOX_WORKSPACE,
        ])
        .args(home_mounts(run))
        .args([
            "--entrypoint",
            BRIDGE,
            image,
            BASH,
            "-c",
            &last_chance_command(),
        ])
        .output();

    match output {
        Ok(output) => {
            // The receive hook answers in the push output, which git relays on
            // the error stream, so both are worth keeping.
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            for line in stdout.lines().chain(stderr.lines()) {
                log::info!("{run}: last chance: {line}");
            }
        }
        Err(error) => log::warn!("{run}: the last chance did not run: {error}"),
    }
}

/// Drop a container, ignoring one that is already gone.
fn remove_container(container: &str) {
    let _ = std::process::Command::new("docker")
        .args(["rm", "--force", container])
        .output();
}

/// Create the egress network when it does not exist yet, once before any run
/// looks for it.
fn ensure_egress_network() -> std::io::Result<()> {
    if !exists(&["network", "inspect", NETWORK_EGRESS])? {
        log::info!("creating the {NETWORK_EGRESS} network");
        process::run_and_assume_success("docker", &["network", "create", NETWORK_EGRESS])?;
    }

    Ok(())
}

/// Start the proxy sidecar belonging to one run on the egress network.
///
/// The sidecar and its socket volume are named after the run, so a run reaches
/// its own proxy and nothing else. No ports are published and no second network
/// is joined, which leaves the socket volume as the only address the sandbox
/// has. Each run gets a fresh volume, so nginx never finds a socket path it
/// cannot bind.
fn start_proxy(run: &str) -> std::io::Result<()> {
    let hosts = read_only_mount(PROXY_HOSTS, CONTAINER_HOSTS)?;
    let container = proxy_container(run);
    log::info!("starting the proxy sidecar {container}");

    process::run_and_assume_success(
        "docker",
        &[
            "run",
            "--detach",
            "--name",
            &container,
            "--network",
            NETWORK_EGRESS,
            "--volume",
            &format!("{}:{SOCKET_DIRECTORY}", socket_volume(run)),
            "--volume",
            &hosts,
            PROXY_IMAGE,
        ],
    )?;

    await_socket(&container, SOCKET_PATH)
}

/// Start the scoring server of one run, sharing the socket volume and no
/// network, so the submissions it executes stay as contained as the final
/// scoring. The entry the run attacks, if any, is mounted in read only.
///
/// The proxy routes requests for the score host onto its socket.
fn start_score_server(
    run: &str,
    game: &str,
    image: &str,
    challenge: Option<&Challenge>,
) -> std::io::Result<()> {
    let container = scorer_container(run);
    log::info!("starting the scoring server {container}");

    let task = read_only_mount(
        &task_directory(game, challenge.is_some())
            .display()
            .to_string(),
        TASK_MOUNT,
    )?;
    let readme = read_only_mount(
        &format!("{GAMES_DIRECTORY}/{TASK_INSTRUCTIONS}"),
        README_MOUNT,
    )?;

    let mut arguments: Vec<String> = [
        "run",
        "--detach",
        "--name",
        &container,
        "--network",
        "none",
        "--ulimit",
        NO_CORE_DUMPS,
        "--user",
        ROOT_USER,
        "--volume",
        &format!("{}:{SOCKET_DIRECTORY}", socket_volume(run)),
        "--volume",
        &task,
        "--volume",
        &readme,
    ]
    .iter()
    .map(|argument| argument.to_string())
    .collect();
    if let Some(challenge) = challenge {
        arguments.push("--volume".to_string());
        arguments.push(challenge_mount(challenge)?);
    }
    arguments.extend(
        ["--entrypoint", BASH, image, SCORE_ENTRY, game]
            .iter()
            .map(|argument| argument.to_string()),
    );
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();

    process::run_and_assume_success("docker", &arguments)?;

    await_socket(&container, SCORE_SOCKET_PATH)
}

/// The mount putting the entry a run attacks into the scoring container, under
/// the name it was kept by.
fn challenge_mount(challenge: &Challenge) -> std::io::Result<String> {
    let name = challenge.path.file_name().ok_or_else(|| {
        std::io::Error::other(format!("{} has no file name", challenge.path.display()))
    })?;
    let source = std::env::current_dir()?.join(&challenge.path);

    Ok(format!(
        "{}:{CHALLENGE_MOUNT}/{}{READ_ONLY}",
        source.display(),
        name.to_string_lossy()
    ))
}

/// Wait until a sidecar has bound the socket the sandbox is about to connect
/// to.
///
/// Docker reports the container as started before anything binds, so a
/// sandbox starting immediately would find nothing at the other end of the
/// mount.
fn await_socket(container: &str, socket: &str) -> std::io::Result<()> {
    for _ in 0..READY_ATTEMPTS {
        if exists(&["exec", container, "test", "-S", socket])? {
            log::debug!("{container} bound {socket}");
            return Ok(());
        }
        std::thread::sleep(READY_INTERVAL);
    }

    Err(std::io::Error::other(format!(
        "{container} did not bind {socket}"
    )))
}

/// Wait out a scoring still in flight when the sandbox died, so an attempt
/// posted right before the end reaches the log before the log is collected.
///
/// The scoring server answers requests one at a time, which makes one served
/// probe the proof that the previous scoring finished. Failures are ignored,
/// since a server that cannot answer has nothing in flight to wait for.
fn drain_scorer(run: &str) {
    let _ = std::process::Command::new("docker")
        .args([
            "exec",
            &scorer_container(run),
            "curl",
            "-sf",
            "--max-time",
            DRAIN_TIMEOUT_SECONDS,
            "--unix-socket",
            SCORE_SOCKET_PATH,
            DRAIN_PROBE_URL,
        ])
        .output();
}

/// Write a sidecar's logs under `runs/<run>` before the container is removed.
///
/// Both sidecars log to the container output, so removing them is what makes
/// the logs unrecoverable. The proxy access log records every call a run
/// made, and the scoring log records every submission it had scored.
fn collect_logs(
    run: &str,
    container: &str,
    stdout_file: &str,
    stderr_file: &str,
) -> std::io::Result<()> {
    let directory = std::path::Path::new(RUN_DIRECTORY).join(run);
    std::fs::create_dir_all(&directory)?;

    let logs = std::process::Command::new("docker")
        .args(["logs", container])
        .output()?;

    std::fs::write(directory.join(stdout_file), logs.stdout)?;
    std::fs::write(directory.join(stderr_file), logs.stderr)?;
    log::info!(
        "collected the {container} logs into {}",
        directory.display()
    );

    Ok(())
}

/// The immutable id of the image behind `tag`.
///
/// A rebuild moves the tag, so every container of one run is started from the id
/// resolved once. That way a run started while another one rebuilds the image
/// still plays, and reports, the harness it began with.
fn image_id(tag: &str) -> std::io::Result<String> {
    let id = process::run_and_assume_success(
        "docker",
        &["image", "inspect", "--format", IMAGE_ID_FORMAT, tag],
    )?;

    log::info!("{tag} is {id}");

    Ok(id)
}

/// Write down what the run was started with, before anything runs.
///
/// Credentials are named but never written, so the record says which
/// variables the sandbox was given without carrying their values.
fn record_run(run: &str, launch: &Launch, harness_version: &str) -> std::io::Result<()> {
    let command = &launch.command;
    let record = ava_wire::Run {
        version: ava_wire::VERSION,
        run: run.to_string(),
        harness: command.name.clone(),
        harness_version: harness_version.to_string(),
        model: command.model.clone(),
        thinking: command.thinking.clone(),
        game: command.game.clone(),
        game_version: launch.game_version.clone(),
        architecture: launch.architecture.clone(),
        limit_seconds: command.limit,
        started_seconds: crate::usage::epoch_now(),
        image: launch.identity.clone(),
        prompt: TASK_PROMPT.to_string(),
        arguments: launch.invocation.arguments.clone(),
        variables: launch
            .invocation
            .variables
            .iter()
            .map(|(name, _)| name.clone())
            .collect(),
        challenge: command
            .challenge
            .as_ref()
            .map(|challenge| challenge.record.clone()),
        finished_seconds: None,
        attempts: Vec::new(),
        metrics: None,
    };

    write_run(run, &record)
}

/// Write `record` as the record of `run`.
pub fn write_run(run: &str, record: &ava_wire::Run) -> std::io::Result<()> {
    let directory = std::path::Path::new(RUN_DIRECTORY).join(run);
    std::fs::create_dir_all(&directory)?;

    std::fs::write(
        directory.join(RUN_FILE),
        format!(
            "{}\n",
            serde_json::to_string_pretty(record).map_err(std::io::Error::other)?
        ),
    )
}

/// The commit the folder of `game`, and the folder of its image, was last
/// changed in, marked when the working tree differs from it. Empty outside a
/// Whether the entries of `game` are attacked by agents, which is what an
/// attack task in its folder says.
pub fn attacked(game: &str) -> bool {
    std::path::Path::new(GAMES_DIRECTORY)
        .join(game)
        .join(ATTACK_DIRECTORY)
        .is_dir()
}

/// The folder holding the task a run of `game` gets: the attack task when the
/// run `attacks` an entry, else the defence when the game has one, else the
/// task.
pub fn task_directory(game: &str, attacks: bool) -> std::path::PathBuf {
    let folder = std::path::Path::new(GAMES_DIRECTORY).join(game);
    if attacks {
        return folder.join(ATTACK_DIRECTORY);
    }

    let defend = folder.join(DEFEND_DIRECTORY);
    if defend.is_dir() {
        return defend;
    }

    folder.join(TASK_DIRECTORY)
}

/// repository.
pub fn game_version(game: &str) -> String {
    let mut folders = vec![format!("{GAMES_DIRECTORY}/{game}")];
    if let Some(layer) = game_layer(game) {
        folders.push(format!("{GAMES_DIRECTORY}/{layer}"));
    }
    let folders: Vec<&str> = folders.iter().map(String::as_str).collect();

    let mut log = vec!["log", "-1", "--format=%h", "--"];
    log.extend(&folders);
    let Ok(commit) = process::run_and_assume_success("git", &log) else {
        return String::new();
    };

    let mut status = vec!["status", "--porcelain", "--"];
    status.extend(&folders);
    let dirty =
        process::run_and_assume_success("git", &status).is_ok_and(|changes| !changes.is_empty());

    if dirty {
        format!("{commit}{DIRTY_SUFFIX}")
    } else {
        commit
    }
}

/// Tag the image for this run, and report the tag.
///
/// A parallel run rebuilding the same harness moves the shared tag, and
/// docker may prune the then untagged image mid run.
fn pin_image(identity: &str, agent: &str, run: &str) -> std::io::Result<String> {
    let pinned = format!("{AGENT_IMAGE_PREFIX}{agent}:{run}");
    process::run_and_assume_success("docker", &["tag", identity, &pinned])?;

    Ok(pinned)
}

/// Drop the tag once the run no longer needs the image.
fn unpin_image(pinned: &str) {
    log::debug!("dropping the image tag {pinned}");

    let _ = std::process::Command::new("docker")
        .args(["rmi", "--no-prune", pinned])
        .output();
}

/// What the harness in `image` reports as its version.
fn harness_version(image: &str) -> std::io::Result<String> {
    let version = process::run_and_assume_success(
        "docker",
        &["run", "--rm", "--network", "none", image, VERSION_OPTION],
    )?;

    log::info!("harness version: {version}");

    Ok(version)
}

/// Copy the entries the scorer kept into `runs/<run>/entries` before the
/// container is removed.
fn collect_entries(run: &str) -> std::io::Result<()> {
    let directory = std::path::Path::new(RUN_DIRECTORY)
        .join(run)
        .join(ENTRIES_DIRECTORY);
    std::fs::create_dir_all(&directory)?;

    process::run_and_assume_success(
        "docker",
        &[
            "cp",
            &format!("{}:{SCORER_ENTRIES}/.", scorer_container(run)),
            &directory.display().to_string(),
        ],
    )?;

    Ok(())
}

/// Remove the sidecars and the socket volume once the run is over.
///
/// Failures are ignored so that teardown cannot mask the status the agent
/// exited with, and a scoring container that never ran is simply not there
/// to remove.
fn remove_sidecars(run: &str) {
    log::debug!("removing the sidecars and socket volume of {run}");

    for arguments in [
        vec!["rm", "--force", &scorer_container(run)],
        vec!["rm", "--force", &proxy_container(run)],
        vec!["rm", "--force", &holder_container(run)],
        vec!["volume", "rm", "--force", &socket_volume(run)],
        vec!["volume", "rm", "--force", &home_volume(run)],
        vec!["volume", "rm", "--force", &workspace_volume(run)],
    ] {
        let _ = std::process::Command::new("docker")
            .args(arguments)
            .output();
    }
}

/// Build the requested docker images, or every image when none is named.
///
/// The base image comes first, since the harness images build on it. The
/// remaining images are independent of each other and build in parallel, with
/// the output of each kept whole and shown only when its build fails. Unlike
/// a run, this always builds, which is how a changed image reaches the next
/// run.
pub fn build_images(command: &Image) -> std::io::Result<i32> {
    let registry = crate::registry::load()?;
    let everything = command.agent.is_empty() && !command.proxy && !command.scorer;

    let harnesses: Vec<String> = if !command.agent.is_empty() {
        vec![registry.harness(&command.agent)?.name.clone()]
    } else if everything {
        registry
            .harnesses
            .iter()
            .map(|harness| harness.name.clone())
            .collect()
    } else {
        Vec::new()
    };

    if !harnesses.is_empty() {
        build_image(BASE_IMAGE, BASE_CONTEXT, true)?;
    }

    let mut builds: Vec<(String, Vec<String>)> = harnesses
        .iter()
        .map(|harness| {
            let tag = format!("{AGENT_IMAGE_PREFIX}{harness}");
            let arguments = vec![
                "build".to_string(),
                "--tag".to_string(),
                tag.clone(),
                format!("{AGENT_CONTEXT}/{harness}"),
            ];
            (tag, arguments)
        })
        .collect();

    if everything || command.proxy {
        builds.push((
            PROXY_IMAGE.to_string(),
            ["build", "--tag", PROXY_IMAGE, PROXY_CONTEXT]
                .map(String::from)
                .to_vec(),
        ));
    }

    if everything || command.scorer {
        builds.push((
            SCORER_IMAGE.to_string(),
            [
                "build",
                "--tag",
                SCORER_IMAGE,
                "--file",
                SCORER_DOCKERFILE,
                REPOSITORY_CONTEXT,
            ]
            .map(String::from)
            .to_vec(),
        ));
    }

    // A single build keeps the live output a parallel run cannot have.
    if let [(tag, arguments)] = builds.as_slice() {
        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
        docker_build(tag, &arguments)?;
    } else {
        let failures: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = builds
                .iter()
                .map(|(tag, arguments)| {
                    scope.spawn(move || {
                        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
                        docker_build_captured(tag, &arguments).map_err(|error| error.to_string())
                    })
                })
                .collect();

            handles
                .into_iter()
                .filter_map(|handle| handle.join().expect("the build threads do not panic").err())
                .collect()
        });

        if !failures.is_empty() {
            return Err(std::io::Error::other(failures.join(", ")));
        }
    }

    let mut layers: Vec<(String, String, &str)> = Vec::new();
    for layer in game_layers() {
        for harness in &harnesses {
            let base = format!("{AGENT_IMAGE_PREFIX}{harness}");
            layers.push((format!("{base}-{layer}"), base, layer));
        }
        if everything || command.scorer {
            layers.push((
                format!("{SCORER_IMAGE}-{layer}"),
                SCORER_IMAGE.to_string(),
                layer,
            ));
        }
    }
    build_game_images(&layers)
}

/// The scorer image of the named game: the scorer with the game's layer, if it has one.
pub fn scorer_image(game: &str) -> String {
    match game_layer(game) {
        Some(layer) => format!("{SCORER_IMAGE}-{layer}"),
        None => SCORER_IMAGE.to_string(),
    }
}

/// The folder whose Dockerfile the named game plays on, if it needs one.
fn game_layer(game: &str) -> Option<&'static str> {
    ava_game::find(game).and_then(|game| game.image())
}

/// Every folder a game layers its software from, each once.
fn game_layers() -> Vec<&'static str> {
    let mut layers: Vec<&str> = ava_game::GAMES
        .iter()
        .filter_map(|game| game.image())
        .collect();
    layers.sort_unstable();
    layers.dedup();
    layers
}

/// Build every `(tag, base, layer)` game image in parallel.
fn build_game_images(layers: &[(String, String, &str)]) -> std::io::Result<i32> {
    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = layers
            .iter()
            .map(|(tag, base, layer)| {
                scope.spawn(move || {
                    let file = format!("{GAME_CONTEXT}/{layer}/{GAME_DOCKERFILE}");
                    let argument = format!("{BASE_ARGUMENT}={base}");
                    docker_build_captured(
                        tag,
                        &[
                            "build",
                            "--tag",
                            tag,
                            "--file",
                            &file,
                            "--build-arg",
                            &argument,
                            GAME_CONTEXT,
                        ],
                    )
                    .map_err(|error| error.to_string())
                })
            })
            .collect();

        handles
            .into_iter()
            .filter_map(|handle| handle.join().expect("the build threads do not panic").err())
            .collect()
    });

    if !failures.is_empty() {
        return Err(std::io::Error::other(failures.join(", ")));
    }

    Ok(0)
}

/// Build the Dockerfile of the `layer` folder over `base` as `tag`, or keep it
/// when it already exists and the build is not forced.
fn build_game_image(tag: &str, base: &str, layer: &str, force: bool) -> std::io::Result<()> {
    if !force && exists(&["image", "inspect", tag])? {
        log::info!("{tag} exists, keeping it");
        return Ok(());
    }

    docker_build(
        tag,
        &[
            "build",
            "--tag",
            tag,
            "--file",
            &format!("{GAME_CONTEXT}/{layer}/{GAME_DOCKERFILE}"),
            "--build-arg",
            &format!("{BASE_ARGUMENT}={base}"),
            GAME_CONTEXT,
        ],
    )
}

/// What one run needs resolved before it plays: its command, the image it
/// plays on, the invocation opening its session and the version of its game.
///
/// Preparing builds what is missing once, so runs started together cannot
/// race each other rebuilding, and every run then plays on the image id
/// resolved here, whatever gets rebuilt in the meantime.
pub struct Launch {
    pub command: Agent,
    /// The immutable id of the harness image, with the layer of the game when it has one.
    pub identity: String,
    invocation: crate::registry::Invocation,
    pub game_version: String,
    /// The architecture of the docker host, as it reports it.
    pub architecture: String,
}

/// Resolve `command` into a launch, building the images, the proxy hosts file
/// and the egress network it needs.
pub fn prepare(command: &Agent) -> std::io::Result<Launch> {
    let agent = command.name.as_str();
    Agent::checked_limit(command.limit)?;
    require_game(&command.game, command.challenge.as_ref())?;

    let registry = crate::registry::load()?;
    let invocation = registry.invocation(
        agent,
        &command.model,
        TASK_PROMPT,
        command.thinking.as_deref(),
        crate::registry::Start::Task,
    )?;

    let force = command.force_build_images;
    build_image(BASE_IMAGE, BASE_CONTEXT, force)?;

    let harness = format!("{AGENT_IMAGE_PREFIX}{agent}");
    build_image(&harness, &format!("{AGENT_CONTEXT}/{agent}"), force)?;
    build_scorer_image(force)?;

    let played = match game_layer(&command.game) {
        Some(layer) => {
            let played = format!("{harness}-{layer}");
            build_game_image(&played, &harness, layer, force)?;
            build_game_image(&scorer_image(&command.game), SCORER_IMAGE, layer, force)?;
            played
        }
        None => harness,
    };
    let identity = image_id(&played)?;

    std::fs::write(PROXY_HOSTS, crate::upstreams::nginx_map(&registry.hosts()))?;
    build_image(PROXY_IMAGE, PROXY_CONTEXT, force)?;
    ensure_egress_network()?;

    Ok(Launch {
        command: command.clone(),
        identity,
        invocation,
        game_version: game_version(&command.game),
        architecture: process::run_and_assume_success(
            "docker",
            &["info", "--format", ARCHITECTURE_FORMAT],
        )?,
    })
}

/// Run the named agent, one run per requested parallel slot, and report the
/// first failing status.
pub fn run_agent(command: &Agent) -> std::io::Result<i32> {
    let launch = prepare(command)?;
    let base = run_name(&command.name);

    if command.parallel == 1 {
        return play(&launch, &base);
    }

    let outcomes: Vec<std::io::Result<i32>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (1..=command.parallel)
            .map(|index| {
                let run = format!("{base}-{index}");
                let launch = &launch;
                scope.spawn(move || play(launch, &run))
            })
            .collect();

        handles
            .into_iter()
            .map(|handle| handle.join().expect("the run threads do not panic"))
            .collect()
    });

    let mut code = 0;
    for outcome in outcomes {
        let finished = outcome?;
        if code == 0 {
            code = finished;
        }
    }

    Ok(code)
}

/// Play one prepared run under the name `run`, against a proxy of its own,
/// and report its status.
///
/// The sidecars live and die with the run: `ava` starts them, runs the sandbox
/// against them and removes everything afterwards. Since the proxy serves one
/// run, its access log describes that run alone.
pub fn play(launch: &Launch, run: &str) -> std::io::Result<i32> {
    let command = &launch.command;
    let staging = std::env::temp_dir().join(STAGING_DIRECTORY).join(run);
    let image = pin_image(&launch.identity, &command.name, run)?;

    log::info!(
        "run {run}: {} on {} playing {}",
        command.name,
        command.model,
        command.game
    );

    // A sidecar that fails to start must not skip the teardown below, or the
    // ones already started leak, so the whole startup lands in one status.
    let status = harness_version(&image)
        .and_then(|version| record_run(run, launch, &version))
        .and_then(|()| start_proxy(run))
        .and_then(|()| {
            start_score_server(
                run,
                &command.game,
                &scorer_image(&command.game),
                command.challenge.as_ref(),
            )
        })
        .and_then(|()| prepare_agent_home(run, &image))
        .and_then(|()| run_sandbox(command, &image, run, &staging, launch.invocation.clone()));

    let collected = collect_logs(run, &proxy_container(run), ACCESS_LOG, ERROR_LOG);
    drain_scorer(run);
    let attempts = collect_logs(run, &scorer_container(run), SCORE_LOG, SCORE_ERROR_LOG);
    let entries = collect_entries(run);
    remove_sidecars(run);

    let completed = match (&status, &collected, &attempts) {
        (Ok(_), Ok(()), Ok(())) => complete_run(run),
        _ => Ok(()),
    };

    unpin_image(&image);

    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }

    let code = status?;
    collected?;
    attempts?;
    entries?;
    completed?;

    if let Some(analyst) = &command.analyst
        && let Err(error) = analyze(&Analyze {
            run: run.to_string(),
            analyst: analyst.clone(),
        })
    {
        log::error!("{run}: the analysis failed: {error}");
    }

    Ok(code)
}

/// Fail before anything is built when the game or its task folder is missing,
/// or when the game attacks the entries of another and no `challenge` is given.
fn require_game(game: &str, challenge: Option<&Challenge>) -> std::io::Result<()> {
    if ava_game::find(game).is_none() {
        return Err(crate::registry::unknown(
            game,
            "game",
            ava_game::GAMES.iter().map(|game| game.name()),
        ));
    }

    if challenge.is_some() && !attacked(game) {
        return Err(std::io::Error::other(format!(
            "{game} has no attack task, nothing attacks its entries"
        )));
    }

    let task = task_directory(game, challenge.is_some());
    if !task.is_dir() {
        return Err(std::io::Error::other(format!(
            "the task folder {} does not exist",
            task.display()
        )));
    }

    if let Some(layer) = game_layer(game) {
        let dockerfile = std::path::Path::new(GAMES_DIRECTORY)
            .join(layer)
            .join(GAME_DOCKERFILE);
        if !dockerfile.is_file() {
            return Err(std::io::Error::other(format!(
                "the game image {} does not exist",
                dockerfile.display()
            )));
        }
    }

    Ok(())
}

/// Complete the record of `run` with what it left behind: every attempt the
/// scorer graded and the metrics of every request the proxy carried.
///
/// Every submission was verified the moment it was pushed, so this only reads
/// logs and nothing executes here.
fn complete_run(run: &str) -> std::io::Result<()> {
    let directory = std::path::Path::new(RUN_DIRECTORY).join(run);
    log::info!("completing the record of {run}");

    let mut record = crate::runs::read(&directory)?;
    record.attempts =
        ava_scorer::score::read_attempts(&directory.join(SCORE_LOG).display().to_string())?;
    record.metrics = Some(ava_scorer::score::aggregate_metrics(
        &directory.join(ACCESS_LOG).display().to_string(),
    )?);
    record.finished_seconds = Some(crate::usage::epoch_now());

    write_run(run, &record)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&record).map_err(std::io::Error::other)?
    );

    Ok(())
}

/// Fight the entry at `first` against the entry at `second` in the scorer
/// image of `game`, with no network and the entries mounted read only, and
/// report the rounds from the view of `first`. What the fight printed is
/// appended to the file at `log`.
pub fn fight(
    game: &str,
    first: &std::path::Path,
    second: &std::path::Path,
    combats: u64,
    log: &std::path::Path,
) -> std::io::Result<ava_wire::Tally> {
    let played = ava_game::find(game).ok_or_else(|| {
        crate::registry::unknown(game, "game", ava_game::GAMES.iter().map(|game| game.name()))
    })?;

    let staging = std::env::temp_dir().join(format!(
        "{FIGHT_STAGING_PREFIX}{}-{}",
        std::process::id(),
        FIGHT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    for (directory, entry) in [
        (ava_scorer::score::FIRST_DIRECTORY, first),
        (ava_scorer::score::SECOND_DIRECTORY, second),
    ] {
        let target = staging.join(directory);
        std::fs::create_dir_all(&target)?;
        std::fs::copy(entry, target.join(played.entry()))?;
    }

    let output = std::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            "none",
            "--ulimit",
            NO_CORE_DUMPS,
            "--user",
            SANDBOX_USER,
            "--env",
            &format!("{HOME_VARIABLE}={AGENT_HOME}"),
            "--volume",
            &format!("{}:{FIGHT_MOUNT}{READ_ONLY}", staging.display()),
            "--workdir",
            AGENT_HOME,
            "--entrypoint",
            TIMEOUT,
            &scorer_image(game),
            FIGHT_TIMEOUT_SECONDS,
            AVA,
            "score",
            "--game",
            game,
            "--fight",
            FIGHT_MOUNT,
            "--combats",
            &combats.to_string(),
        ])
        .output();
    let _ = std::fs::remove_dir_all(&staging);
    let output = output?;

    let mut console = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)?;
    std::io::Write::write_all(
        &mut console,
        format!("fight: {} against {}\n", first.display(), second.display()).as_bytes(),
    )?;
    std::io::Write::write_all(&mut console, &output.stderr)?;

    if !output.status.success() {
        let reason = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::other(format!(
            "the fight failed: {}",
            reason.lines().last().unwrap_or_default().trim()
        )));
    }

    #[derive(serde::Deserialize)]
    struct Fought {
        fight: ava_wire::Tally,
    }
    let fought: Fought = serde_json::from_slice(&output.stdout).map_err(|error| {
        std::io::Error::other(format!("the fight report does not parse: {error}"))
    })?;

    Ok(fought.fight)
}

/// Run the sandbox itself, without a network, and report its status.
///
/// The sandbox gets no network interface beyond loopback, so the socket volume
/// is its only way out and no two runs can see each other. Every backend host
/// is pinned to loopback in the container host file, where the bridge started
/// by the image entrypoint forwards to the proxy socket.
///
/// Nothing survives the run: submissions leave through the scoring server and
/// the home is a volume of the run's own, removed with it, so no harness can
/// observe another one and no run inherits leftovers. Credentials are set on
/// the child rather than passed as arguments, which keeps them out of the host
/// process list.
///
/// The run is a loop of turns. `ava` prompts the harness, the harness answers
/// one turn and exits, and that exit is the boundary the next turn starts on,
/// continuing the session the turn before it recorded. Every harness takes
/// this same path, so the loop is one thing rather than one per harness, and
/// the turn count is something the run records instead of something only the
/// harness knows.
///
/// The clock bounds the whole loop, not a turn: each start is given whatever
/// is left of the run. Running out of it is the last call.
fn run_sandbox(
    command: &Agent,
    image: &str,
    run: &str,
    staging: &std::path::Path,
    invocation: crate::registry::Invocation,
) -> std::io::Result<i32> {
    let sandbox = Sandbox::run(command, image, run, staging);
    let loop_limit = loop_seconds(command.limit);
    let mut phase = Phase {
        limit: loop_limit,
        started: std::time::Instant::now(),
        loop_limit,
        run_limit: command.limit,
        last_call: false,
        turn: 1,
        monitor: std::sync::Arc::new(crate::monitor::Monitor::new()),
    };

    // A budget of exactly the last call leaves the loop nothing to spend.
    if loop_limit == 0 {
        log::info!("{run}: the budget covers the last call and nothing before it");
        phase.turn = 0;
        return last_call(&sandbox, &phase);
    }

    match turn_loop(&sandbox, invocation, &mut phase, &|| Ok(true))? {
        Ending::Done(code) => Ok(code),
        Ending::OutOfTime(_) | Ending::TurnOver(_) => last_call(&sandbox, &phase),
    }
}

/// Start the agent one final turn, prompted to submit instead of to carry on.
///
/// The home is a volume, so the harness session and the workspace are both
/// still there and this turn continues the conversation like any other. The
/// only thing that sets it apart is the prompt and that no turn follows it.
fn last_call(sandbox: &Sandbox, task: &Phase) -> std::io::Result<i32> {
    let prompt = last_call_prompt();
    let invocation = crate::registry::load()?.invocation(
        &sandbox.agent.harness,
        &sandbox.agent.model,
        &prompt,
        sandbox.agent.thinking.as_deref(),
        crate::registry::Start::Resume,
    )?;

    log::info!("{}: starting the agent for the last call", sandbox.name);

    let phase = Phase {
        limit: LAST_CALL_SECONDS,
        started: task.started,
        loop_limit: task.loop_limit,
        run_limit: task.run_limit,
        last_call: true,
        turn: task.turn + 1,
        monitor: task.monitor.clone(),
    };

    let ending = start_sandbox(sandbox, &invocation, &phase);

    // Whatever the agent left is worth submitting even when the last call
    // itself never got going.
    last_chance(&sandbox.name, &sandbox.image);

    match ending? {
        Ending::TurnOver(code) | Ending::Done(code) | Ending::OutOfTime(code) => Ok(code),
    }
}

/// Why the sandbox stopped.
enum Ending {
    /// The harness exited, which ends a turn rather than the run. The next
    /// turn continues the session this one recorded.
    TurnOver(i32),
    /// The agent reported itself done or the run was interrupted, so no turn
    /// follows and there is nothing left to ask for.
    Done(i32),
    /// The clock ran out, so the agent still gets its last call.
    OutOfTime(i32),
}

/// A sandbox: a run's or an analyst's.
struct Sandbox {
    agent: ava_wire::Agent,
    task: &'static str,
    /// What the sidecars and volumes are named after.
    name: String,
    /// The run directory the console goes to.
    directory: String,
    console: &'static str,
    heartbeat: Option<&'static str>,
    image: String,
    staging: std::path::PathBuf,
    mounts: Vec<String>,
    /// Whether a scoring container serves the sandbox.
    scored: bool,
}

impl Sandbox {
    /// The sandbox of a run.
    fn run(command: &Agent, image: &str, run: &str, staging: &std::path::Path) -> Self {
        Self {
            agent: command.agent(),
            task: TASK_PROMPT,
            name: run.to_string(),
            directory: run.to_string(),
            console: AGENT_LOG,
            heartbeat: Some(MONITOR_FILE),
            image: image.to_string(),
            staging: staging.to_path_buf(),
            mounts: Vec::new(),
            scored: true,
        }
    }

    fn container(&self) -> String {
        sandbox_container(&self.name)
    }
}

/// Start turns until the clock runs out or `unfinished` says no.
fn turn_loop(
    sandbox: &Sandbox,
    first: crate::registry::Invocation,
    phase: &mut Phase,
    unfinished: &dyn Fn() -> std::io::Result<bool>,
) -> std::io::Result<Ending> {
    let registry = crate::registry::load()?;
    let mut turn = first;

    loop {
        let entered = std::time::Instant::now();
        let ending = start_sandbox(sandbox, &turn, phase)?;
        let Ending::TurnOver(code) = ending else {
            return Ok(ending);
        };

        if !unfinished()? {
            return Ok(Ending::Done(code));
        }

        let Some(left) = phase.remaining() else {
            log::info!("{}: the clock ran out as the turn ended", sandbox.name);
            return Ok(Ending::OutOfTime(code));
        };

        // A harness failing at startup would otherwise spin through the clock.
        let spent = entered.elapsed().as_secs();
        if spent < TURN_RETRY_SECONDS {
            log::warn!(
                "{}: turn {} ended after {spent} seconds with {code}, waiting to start the next",
                sandbox.name,
                phase.turn
            );
            std::thread::sleep(std::time::Duration::from_secs(TURN_RETRY_SECONDS));
        }

        phase.turn += 1;
        phase.limit = left;
        turn = registry.invocation(
            &sandbox.agent.harness,
            &sandbox.agent.model,
            &loop_prompt(phase.turn, sandbox.task),
            sandbox.agent.thinking.as_deref(),
            crate::registry::Start::Resume,
        )?;
    }
}

/// Start one turn and wait for it.
fn start_sandbox(
    sandbox: &Sandbox,
    invocation: &crate::registry::Invocation,
    phase: &Phase,
) -> std::io::Result<Ending> {
    let harness = sandbox.agent.harness.as_str();
    let container = sandbox.container();

    let mut docker = std::process::Command::new("docker");
    docker.args([
        "run",
        "--rm",
        "--name",
        &container,
        "--network",
        "none",
        "--ulimit",
        NO_CORE_DUMPS,
        "--ulimit",
        MAX_FILE_BYTES,
        "--read-only",
        "--tmpfs",
        &format!("{SANDBOX_TMPFS},{SCRATCH_SIZE}"),
        "--workdir",
        SANDBOX_WORKSPACE,
        "--volume",
        &format!(
            "{}:{SOCKET_DIRECTORY}{READ_ONLY}",
            socket_volume(&sandbox.name)
        ),
    ]);

    docker.args(home_mounts(&sandbox.name));

    docker.args(["--hostname", harness]);
    docker.args(["--add-host", &format!("{harness}:{SANDBOX_LOOPBACK}")]);

    for host in &invocation.hosts {
        docker.args(["--add-host", &format!("{host}:{SANDBOX_LOOPBACK}")]);
    }

    if sandbox.scored {
        docker.args(["--add-host", &format!("{GIT_HOST}:{SANDBOX_LOOPBACK}")]);
        docker.args(["--add-host", &format!("{SCORER_HOST}:{SANDBOX_LOOPBACK}")]);
    }

    for mount in &sandbox.mounts {
        docker.args(["--volume", mount]);
    }

    for (path, contents) in &invocation.files {
        docker.args(["--volume", &staged_mount(&sandbox.staging, path, contents)?]);
    }

    for (name, value) in &invocation.variables {
        docker.args(["--env", name]);
        docker.env(name, value);
    }

    // Every turn starts a container of the same name, and `--rm` frees that
    // name a moment after the client exits rather than before, so the name is
    // cleared here instead of raced for.
    remove_container(&container);

    log::info!("starting the sandbox {container} from {}", sandbox.image);
    log::debug!("{harness} arguments: {:?}", invocation.arguments);

    docker.stdout(std::process::Stdio::piped());
    docker.stderr(std::process::Stdio::piped());

    // The client gets its own process group, so signals aimed at the terminal
    // cannot kill it and the run ends only when ava ends it.
    std::os::unix::process::CommandExt::process_group(&mut docker, 0);

    let mut child = docker
        .arg(&sandbox.image)
        .args(&invocation.arguments)
        .spawn()?;

    phase.monitor.restart();
    let readers = record_output(
        &mut child,
        &sandbox.directory,
        sandbox.console,
        &phase.monitor,
    )?;
    let ending = await_sandbox(child, sandbox, phase)?;

    for reader in readers {
        reader.join().expect("the reader threads do not panic")?;
    }

    Ok(ending)
}

/// The file keeping what the agent printed, cut at the ceiling.
struct Console {
    run: String,
    file: std::fs::File,
    bytes: u64,
}

impl Console {
    /// Appended rather than truncated, so the last call adds to the console of
    /// the run instead of replacing what the task start wrote.
    fn open(run: &str, name: &str) -> std::io::Result<Self> {
        let directory = std::path::Path::new(RUN_DIRECTORY).join(run);
        std::fs::create_dir_all(&directory)?;

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join(name))?;
        let bytes = file.metadata()?.len();

        Ok(Self {
            run: run.to_string(),
            file,
            bytes,
        })
    }

    fn append(&mut self, chunk: &[u8]) -> std::io::Result<()> {
        if self.bytes > AGENT_LOG_CEILING {
            return Ok(());
        }

        let room = (AGENT_LOG_CEILING - self.bytes) as usize;
        if chunk.len() <= room {
            std::io::Write::write_all(&mut self.file, chunk)?;
            self.bytes += chunk.len() as u64;
            return Ok(());
        }

        std::io::Write::write_all(&mut self.file, &chunk[..room])?;
        std::io::Write::write_all(&mut self.file, AGENT_LOG_CUT.as_bytes())?;
        self.bytes = AGENT_LOG_CEILING + AGENT_LOG_CUT.len() as u64;
        log::warn!(
            "{}: the console reached {AGENT_LOG_CEILING} bytes, cutting it",
            self.run
        );

        Ok(())
    }
}

/// Keep what the agent printed in `runs/<run>/<file>`.
///
/// The terminal gets the periodic status log instead of the console. Only the
/// stdout stream carrying the harness events is scanned for repeated lines.
fn record_output(
    child: &mut std::process::Child,
    run: &str,
    file: &str,
    monitor: &std::sync::Arc<crate::monitor::Monitor>,
) -> std::io::Result<Vec<std::thread::JoinHandle<std::io::Result<()>>>> {
    let stdout = child.stdout.take().expect("the sandbox output is piped");
    let stderr = child.stderr.take().expect("the sandbox output is piped");

    let console = std::sync::Arc::new(std::sync::Mutex::new(Console::open(run, file)?));

    Ok(vec![
        record(stdout, console.clone(), monitor.clone(), true),
        record(stderr, console, monitor.clone(), false),
    ])
}

/// Copy `source` into `console` until it ends, reporting every chunk to the
/// monitor.
fn record(
    mut source: impl std::io::Read + Send + 'static,
    console: std::sync::Arc<std::sync::Mutex<Console>>,
    monitor: std::sync::Arc<crate::monitor::Monitor>,
    scan_lines: bool,
) -> std::thread::JoinHandle<std::io::Result<()>> {
    std::thread::spawn(move || {
        let mut buffer = [0; RECORD_BUFFER_BYTES];

        loop {
            let read = source.read(&mut buffer)?;
            if read == 0 {
                return Ok(());
            }

            monitor.observe(&buffer[..read], scan_lines);
            console
                .lock()
                .expect("the console is not poisoned")
                .append(&buffer[..read])?;
        }
    })
}

/// What one start of the sandbox is bounded by, and where it sits in the run.
///
/// The clock the interface shows counts from the start of the run, not from the
/// start of this phase, or a restart would send it back to zero.
struct Phase {
    /// The seconds this start is given.
    limit: u64,
    /// When the run began.
    started: std::time::Instant,
    /// The seconds the turn loop may spend, the budget less the last call.
    loop_limit: u64,
    /// The seconds the run as a whole was given, the last call included.
    run_limit: u64,
    /// Whether this start is the last call.
    last_call: bool,
    /// Which turn of the run this start is, counted from one.
    turn: u32,
    /// What the agent printed over the run, shared with the reader threads and
    /// carried across a restart so the console it counts is the whole run.
    monitor: std::sync::Arc<crate::monitor::Monitor>,
}

impl Phase {
    /// The seconds the turn loop has left, or nothing once they are spent.
    fn remaining(&self) -> Option<u64> {
        self.loop_limit
            .checked_sub(self.started.elapsed().as_secs())
            .filter(|left| *left > 0)
    }
}

/// What the run loop knows about a live run, for the web interface.
#[derive(serde::Serialize)]
struct Heartbeat {
    elapsed_seconds: u64,
    limit_seconds: u64,
    output_bytes: u64,
    silent_seconds: u64,
    /// Which turn of the run is live, which only the loop knows.
    turn: u32,
    /// Whether the run is past its limit and answering its last call.
    last_call: bool,
}

/// Write the state of the run loop under the run, best effort.
fn record_heartbeat(sandbox: &Sandbox, phase: &Phase) {
    let Some(file) = sandbox.heartbeat else {
        return;
    };

    let heartbeat = Heartbeat {
        elapsed_seconds: phase.started.elapsed().as_secs(),
        limit_seconds: phase.run_limit,
        output_bytes: phase.monitor.output_bytes(),
        silent_seconds: phase.monitor.silent_for().as_secs(),
        turn: phase.turn,
        last_call: phase.last_call,
    };

    if let Ok(contents) = serde_json::to_string(&heartbeat) {
        let _ = std::fs::write(
            std::path::Path::new(RUN_DIRECTORY)
                .join(&sandbox.directory)
                .join(file),
            contents,
        );
    }
}

/// Wait for the sandbox, killing the container once the agent reports itself
/// done, its time is up, or the user interrupts the run.
///
/// A status line is logged every minute, with warnings for an agent gone
/// silent or repeating itself.
///
/// Running out of time ends the phase rather than the run: the task phase
/// reports [`Ending::OutOfTime`] and the agent is started again for its last
/// call.
///
/// Killing the docker client would leave the container running, so the
/// container is what gets killed, and the client exits on its own once it
/// does. An interrupt outranks a dead client, since a Ctrl+C reaches the
/// client too and killing it never killed the container.
fn await_sandbox(
    mut client: std::process::Child,
    sandbox: &Sandbox,
    phase: &Phase,
) -> std::io::Result<Ending> {
    let monitor = phase.monitor.as_ref();
    let name = sandbox.name.as_str();
    let container = sandbox.container();
    let entered = std::time::Instant::now();
    let deadline = entered + std::time::Duration::from_secs(phase.limit);
    let mut next_status = entered + STATUS_INTERVAL;
    let mut warned_silence = std::time::Duration::ZERO;
    let mut warned_looping = false;
    let mut out_of_time = false;
    let scorer = scorer_container(name);
    log::info!(
        "{name}: turn {} of the agent has {} seconds",
        phase.turn,
        phase.limit
    );
    record_heartbeat(sandbox, phase);

    loop {
        let exited = client.try_wait()?;

        if crate::interrupt::interrupted() {
            log::warn!("the run was interrupted, killing {container}");
        } else if let Some(status) = exited {
            log::info!(
                "{name}: turn {} of the agent exited with {status}",
                phase.turn
            );
            return Ok(Ending::TurnOver(status.code().unwrap_or(1)));
        } else if sandbox.scored && exists(&["exec", &scorer, "test", "-f", DONE_MARKER])? {
            log::info!("{name}: the agent reported itself done, stopping {container}");
        } else if std::time::Instant::now() >= deadline {
            log::warn!(
                "{name}: the agent ran out of time after {} seconds, stopping it",
                phase.limit
            );
            out_of_time = true;
        } else {
            let silence = monitor.silent_for();

            if silence < SILENCE_WARNING {
                warned_silence = std::time::Duration::ZERO;
            } else if silence - warned_silence >= SILENCE_WARNING {
                log::warn!(
                    "{name}: the agent printed nothing for {} seconds, maybe a thinking request runs long",
                    silence.as_secs()
                );
                warned_silence = silence;
            }

            if !warned_looping && monitor.doom_looping() {
                log::warn!(
                    "{name}: the agent repeated one output line {} times in a row, it may be stuck in a loop",
                    crate::monitor::REPEATED_LINE_THRESHOLD
                );
                warned_looping = true;
            }

            if std::time::Instant::now() >= next_status {
                log::info!(
                    "{name}: turn {}, {}s of {}s, {} KiB from the agent, the last output {}s ago",
                    phase.turn,
                    phase.started.elapsed().as_secs(),
                    phase.run_limit,
                    monitor.output_bytes() / KIBIBYTE,
                    silence.as_secs()
                );
                record_heartbeat(sandbox, phase);
                next_status += STATUS_INTERVAL;
            }

            std::thread::sleep(CLOCK_INTERVAL);
            continue;
        }

        let _ = std::process::Command::new("docker")
            .args(["kill", &container])
            .output();
        let code = client.wait()?.code().unwrap_or(1);

        return Ok(if out_of_time {
            Ending::OutOfTime(code)
        } else {
            Ending::Done(code)
        });
    }
}

/// Write generated configuration to a staging file and mount it read only.
///
/// The staging directory is named after the running process, so concurrent
/// runs of one harness cannot overwrite each other, and it is removed once the
/// container is gone.
fn staged_mount(
    directory: &std::path::Path,
    path: &str,
    contents: &str,
) -> std::io::Result<String> {
    let name = std::path::Path::new(path)
        .file_name()
        .ok_or_else(|| std::io::Error::other(format!("{path} has no file name")))?;

    std::fs::create_dir_all(directory)?;

    let staged = directory.join(name);
    std::fs::write(&staged, contents)?;

    Ok(format!("{}:{path}{READ_ONLY}", staged.display()))
}

/// Build `tag` from `context`, or keep it when it already exists and the build
/// is not forced.
fn build_image(tag: &str, context: &str, force: bool) -> std::io::Result<()> {
    if !force && exists(&["image", "inspect", tag])? {
        log::info!("{tag} exists, keeping it");
        return Ok(());
    }

    docker_build(tag, &["build", "--tag", tag, context])
}

/// Build the scorer from the repository root, which holds the workspace
/// sources its Dockerfile compiles.
fn build_scorer_image(force: bool) -> std::io::Result<()> {
    if !force && exists(&["image", "inspect", SCORER_IMAGE])? {
        log::info!("{SCORER_IMAGE} exists, keeping it");
        return Ok(());
    }

    docker_build(
        SCORER_IMAGE,
        &[
            "build",
            "--tag",
            SCORER_IMAGE,
            "--file",
            SCORER_DOCKERFILE,
            REPOSITORY_CONTEXT,
        ],
    )
}

fn docker_build(tag: &str, arguments: &[&str]) -> std::io::Result<()> {
    log::info!("building {tag}");

    let status = std::process::Command::new("docker")
        .args(arguments)
        .status()?;

    if !status.success() {
        return Err(std::io::Error::other(format!("building {tag} failed")));
    }

    Ok(())
}

/// Build `tag` with the output captured, for the builds running in parallel.
///
/// The output is shown only when the build fails, whole instead of interleaved
/// with the other builds.
fn docker_build_captured(tag: &str, arguments: &[&str]) -> std::io::Result<()> {
    log::info!("building {tag}");

    let output = std::process::Command::new("docker")
        .args(arguments)
        .output()?;

    if !output.status.success() {
        let mut stderr = std::io::stderr().lock();
        let _ = std::io::Write::write_all(&mut stderr, &output.stdout);
        let _ = std::io::Write::write_all(&mut stderr, &output.stderr);
        return Err(std::io::Error::other(format!("building {tag} failed")));
    }

    log::info!("built {tag}");
    Ok(())
}

/// Read only mounts of the run files and the book.
fn analysis_mounts(run: &str) -> std::io::Result<Vec<String>> {
    let directory = std::env::current_dir()?.join(RUN_DIRECTORY).join(run);
    let mut mounts = Vec::new();

    for entry in std::fs::read_dir(&directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(ANALYSIS_PREFIX) {
            continue;
        }
        mounts.push(format!(
            "{}:{RUN_MOUNT}/{name}{READ_ONLY}",
            entry.path().display()
        ));
    }
    mounts.push(read_only_mount(BOOK_DIRECTORY, BOOK_MOUNT)?);

    Ok(mounts)
}

fn read_only_mount(source: &str, target: &str) -> std::io::Result<String> {
    let source = std::env::current_dir()?.join(source);
    Ok(format!("{}:{target}{READ_ONLY}", source.display()))
}

fn exists(arguments: &[&str]) -> std::io::Result<bool> {
    Ok(std::process::Command::new("docker")
        .args(arguments)
        .output()?
        .status
        .success())
}

/// Analyze a finished run with an agent into `runs/<run>/analysis.json`.
///
/// Turns of the harness on the run and the book, without a git or score host.
pub fn analyze(command: &Analyze) -> std::io::Result<i32> {
    let run = command.run.as_str();
    let directory = std::path::Path::new(RUN_DIRECTORY).join(run);
    if !directory.join(RUN_FILE).is_file() {
        return Err(std::io::Error::other(format!("{run}: no such run")));
    }

    let analyst = &command.analyst;
    Analyst::checked_limit(analyst.limit)?;
    let agent = analyst.agent();
    let registry = crate::registry::load()?;
    let invocation = registry.invocation(
        &agent.harness,
        &agent.model,
        ANALYSIS_PROMPT,
        agent.thinking.as_deref(),
        crate::registry::Start::Task,
    )?;

    build_image(BASE_IMAGE, BASE_CONTEXT, false)?;
    let harness = format!("{AGENT_IMAGE_PREFIX}{}", agent.harness);
    build_image(
        &harness,
        &format!("{AGENT_CONTEXT}/{}", agent.harness),
        false,
    )?;
    std::fs::write(PROXY_HOSTS, crate::upstreams::nginx_map(&registry.hosts()))?;
    build_image(PROXY_IMAGE, PROXY_CONTEXT, false)?;
    ensure_egress_network()?;

    let name = analysis_name(run);
    let identity = image_id(&harness)?;
    let image = pin_image(&identity, &agent.harness, &name)?;
    let _ = std::fs::remove_file(directory.join(ANALYSIS_LOG));

    log::info!("analyzing {run} with {}", agent.label());

    let sandbox = Sandbox {
        agent: agent.clone(),
        task: ANALYSIS_PROMPT,
        name: name.clone(),
        directory: run.to_string(),
        console: ANALYSIS_LOG,
        heartbeat: None,
        image: image.clone(),
        staging: std::env::temp_dir().join(STAGING_DIRECTORY).join(&name),
        mounts: analysis_mounts(run)?,
        scored: false,
    };
    let mut record = ava_wire::Analysis {
        version: ava_wire::VERSION,
        analyst: Some(agent),
        image: identity,
        limit_seconds: analyst.limit,
        started_seconds: crate::usage::epoch_now(),
        ..Default::default()
    };
    let mut phase = Phase {
        limit: analyst.limit,
        started: std::time::Instant::now(),
        loop_limit: analyst.limit,
        run_limit: analyst.limit,
        last_call: false,
        turn: 1,
        monitor: std::sync::Arc::new(crate::monitor::Monitor::new()),
    };

    // Written before the turns, so the run page names the analyst while it
    // runs; completed below with what it left.
    write_analysis(&directory, &record)?;
    let outcome = analysis_turns(&sandbox, invocation, &mut phase, &mut record);

    let collected = collect_report(&holder_container(&name));
    let logged = collect_logs(
        run,
        &proxy_container(&name),
        ANALYSIS_ACCESS_LOG,
        ANALYSIS_ERROR_LOG,
    );
    remove_sidecars(&name);
    unpin_image(&image);
    if sandbox.staging.exists() {
        std::fs::remove_dir_all(&sandbox.staging)?;
    }

    record.turns = phase.turn;
    record.finished_seconds = crate::usage::epoch_now();
    if logged.is_ok() {
        record.metrics = ava_scorer::score::aggregate_metrics(
            &directory.join(ANALYSIS_ACCESS_LOG).display().to_string(),
        )
        .ok();
    }
    match (&outcome, &collected) {
        (Err(error), _) | (Ok(_), Err(error)) => record.error = Some(error.to_string()),
        (Ok(_), Ok(report)) => record.report = Some(report.clone()),
    }
    write_analysis(&directory, &record)?;

    let code = outcome?;
    logged?;
    collected.map_err(|error| std::io::Error::other(format!("{run}: {error}")))?;
    log::info!(
        "{run}: the analysis is in {}",
        directory.join(ANALYSIS_FILE).display()
    );

    Ok(code)
}

/// The sidecars and turns of an analysis.
fn analysis_turns(
    sandbox: &Sandbox,
    first: crate::registry::Invocation,
    phase: &mut Phase,
    record: &mut ava_wire::Analysis,
) -> std::io::Result<i32> {
    record.harness_version = harness_version(&sandbox.image)?;
    start_proxy(&sandbox.name)?;
    prepare_agent_home(&sandbox.name, &sandbox.image)?;

    let holder = holder_container(&sandbox.name);
    let ending = turn_loop(sandbox, first, phase, &|| {
        report_written(&holder).map(|written| !written)
    })?;

    Ok(match ending {
        Ending::TurnOver(code) | Ending::Done(code) | Ending::OutOfTime(code) => code,
    })
}

/// Whether the analyst wrote its report.
fn report_written(holder: &str) -> std::io::Result<bool> {
    exists(&[
        "exec",
        holder,
        "test",
        "-s",
        &format!("{WORKSPACE_STAGE}/{REPORT_FILE}"),
    ])
}

/// The report the analyst wrote, checked.
fn collect_report(holder: &str) -> std::io::Result<ava_wire::Report> {
    let text = process::run_and_assume_success(
        "docker",
        &[
            "exec",
            holder,
            "cat",
            &format!("{WORKSPACE_STAGE}/{REPORT_FILE}"),
        ],
    )
    .map_err(|error| {
        std::io::Error::other(format!("the analyst left no {REPORT_FILE}: {error}"))
    })?;

    checked_report(&text)
}

/// The report in `text`, or why it does not pass as one: a field the analyst
/// has to fill is empty, or a field with a closed vocabulary holds a word
/// outside it.
fn checked_report(text: &str) -> std::io::Result<ava_wire::Report> {
    let report: ava_wire::Report = serde_json::from_str(text).map_err(|error| {
        std::io::Error::other(format!("{REPORT_FILE} is not a report: {error}"))
    })?;

    let required = [
        ("strategy", &report.strategy),
        ("went_well", &report.went_well),
        ("decisive", &report.decisive),
        ("attribution", &report.attribution),
        ("verification", &report.verification),
        ("pacing", &report.pacing),
        ("summary", &report.summary),
        ("analysis", &report.analysis),
    ];
    for (field, value) in required {
        if value.trim().is_empty() {
            return Err(std::io::Error::other(format!(
                "{REPORT_FILE} leaves {field} empty"
            )));
        }
    }

    if ava_wire::meaning(&ava_wire::ATTRIBUTIONS, &report.attribution).is_none() {
        return Err(std::io::Error::other(format!(
            "{REPORT_FILE} attributes the outcome to `{}`, not one of {}",
            report.attribution,
            words(&ava_wire::ATTRIBUTIONS)
        )));
    }
    if !report.failure_mode.is_empty()
        && ava_wire::meaning(&ava_wire::FAILURE_MODES, &report.failure_mode).is_none()
    {
        return Err(std::io::Error::other(format!(
            "{REPORT_FILE} files the failure under `{}`, not one of {}",
            report.failure_mode,
            words(&ava_wire::FAILURE_MODES)
        )));
    }
    if report.failure_mode == ava_wire::OTHER_FAILURE_MODE && report.other_failure.trim().is_empty()
    {
        return Err(std::io::Error::other(format!(
            "{REPORT_FILE} files the failure as {} without naming it in other_failure",
            ava_wire::OTHER_FAILURE_MODE
        )));
    }

    Ok(report)
}

/// The words of a vocabulary, listed.
fn words(vocabulary: &[(&str, &str)]) -> String {
    vocabulary
        .iter()
        .map(|(word, _)| *word)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Write the analysis record.
fn write_analysis(directory: &std::path::Path, record: &ava_wire::Analysis) -> std::io::Result<()> {
    std::fs::write(
        directory.join(ANALYSIS_FILE),
        format!(
            "{}\n",
            serde_json::to_string_pretty(record).map_err(std::io::Error::other)?
        ),
    )
}

#[cfg(test)]
mod tests {
    fn report(attribution: &str, failure_mode: &str, other_failure: &str) -> String {
        format!(
            r#"{{"strategy": "s", "went_well": "w", "decisive": "d", "attribution": "{attribution}",
                "verification": "v", "pacing": "p", "failure_mode": "{failure_mode}",
                "other_failure": "{other_failure}", "summary": "short", "analysis": "long"}}"#
        )
    }

    #[test]
    fn a_report_passes_with_its_fields_filled() {
        let passed = super::checked_report(&report("agent", "unbanked", "")).unwrap();
        assert_eq!(passed.failure_mode, "unbanked");
        assert_eq!(passed.analysis, "long");
        assert!(super::checked_report(&report("environment", "", "")).is_ok());
        assert!(super::checked_report(&report("mixed", "other", "the disk filled")).is_ok());
    }

    #[test]
    fn a_report_fails_on_an_empty_field_or_a_word_outside_the_vocabulary() {
        let empty = super::checked_report("{}").unwrap_err().to_string();
        assert!(empty.contains("strategy"), "{empty}");
        assert!(super::checked_report("not json").is_err());
        assert!(super::checked_report(&report("nobody", "", "")).is_err());
        assert!(super::checked_report(&report("agent", "bad_luck", "")).is_err());
        assert!(super::checked_report(&report("agent", "other", "")).is_err());
    }
}
