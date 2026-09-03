//! Docker orchestration of the containers taking part in a benchmark run.

use crate::process;

/// The agent sandbox command.
#[derive(Debug)]
pub struct Agent {
    /// The agent to run, naming a directory under `agents`.
    pub name: String,
    /// The model the agent runs on.
    pub model: String,
    /// The game to play and score, naming a directory under `games`.
    pub game: String,
    /// The wall clock time the agent is given, in seconds.
    pub limit: u64,
    /// How many runs are started in parallel.
    pub parallel: u64,
    /// How much thinking the agent is asked for.
    pub thinking: Option<String>,
    /// Whether the docker images are rebuilt instead of reused.
    pub force_build_images: bool,
}

impl Agent {
    /// The seconds an agent is given unless the command names a limit.
    pub const DEFAULT_LIMIT_SECONDS: u64 = 300;
    /// The runs started unless the command names a count.
    pub const DEFAULT_PARALLEL_RUNS: u64 = 1;
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
        }
    }
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

const NETWORK_EGRESS: &str = "ava-egress";
const SOCKET_VOLUME_PREFIX: &str = "ava-sockets-";
const SOCKET_DIRECTORY: &str = "/run/ava";
const SOCKET_PATH: &str = "/run/ava/proxy.sock";
const SCORE_SOCKET_PATH: &str = "/run/ava/score.sock";
const PROXY_CONTAINER_PREFIX: &str = "ava-proxy-";
const SCORER_CONTAINER_PREFIX: &str = "ava-scorer-";
const SANDBOX_CONTAINER_PREFIX: &str = "ava-agent-";

/// The host name the proxy routes onto the scoring socket.
const SCORE_HOST: &str = "git";
const SCORE_ENTRY: &str = "/home/agent/score-entry.sh";
const BASH: &str = "bash";
const ROOT_USER: &str = "0";
const PROXY_IMAGE: &str = "ava/openapi-proxy";
const PROXY_CONTEXT: &str = "openapi-proxy";
const PROXY_HOSTS: &str = "openapi-proxy/hosts.conf";
const CONTAINER_HOSTS: &str = "/etc/nginx/conf.d/hosts.conf";
const READ_ONLY: &str = ":ro";
const BASE_IMAGE: &str = "ava/host-env";
const BASE_CONTEXT: &str = "host-env";
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
const RECORD_BUFFER_BYTES: usize = 8 * 1024;

const KIBIBYTE: u64 = 1024;

/// How often the run loop reports the state of the run.
const STATUS_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// Silence long enough to warn about, and to warn about again while it lasts.
const SILENCE_WARNING: std::time::Duration = std::time::Duration::from_secs(120);

/// The marker the receive hook leaves once the agent pushes a release tag.
pub const DONE_MARKER: &str = "/home/agent/done";

/// The seconds the agent gets to answer the last call.
const LAST_CALL_SECONDS: u64 = 120;

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

pub const METADATA_FILE: &str = "run.json";
pub const VERSION_FILE: &str = "harness.version";
const VERSION_OPTION: &str = "--version";
const IMAGE_ID_FORMAT: &str = "{{.Id}}";

const SCORER_IMAGE: &str = "ava/scorer";
const SCORER_DOCKERFILE: &str = "scorer/Dockerfile";
const REPOSITORY_CONTEXT: &str = ".";
const GAMES_DIRECTORY: &str = "games";
const TASK_INSTRUCTIONS: &str = "README.md";

const TASK_MOUNT: &str = "/home/agent/task";
const README_MOUNT: &str = "/home/agent/README.md";
pub const SCORE_FILE: &str = "score.json";

/// The run loop clock as `runs/<run>/monitor.json`, freshened on every status
/// tick. The loop clock is monotonic and pauses with the host, so wall clock
/// arithmetic overstates a live run whenever the host slept.
pub const MONITOR_FILE: &str = "monitor.json";
const READY_ATTEMPTS: u32 = 100;
const READY_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
const CLOCK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// The prompt a harness is started on.
const TASK_PROMPT: &str = "Read README.md in your workspace and work on the task it lays out.";

/// Counts the launches of this process, telling apart the runs a long lived
/// process such as the web interface starts one after another.
static RUN_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The unique name stem of one launch, extending the process id with the
/// launch count once the first launch took the plain name.
fn run_base(agent: &str) -> String {
    let launch = RUN_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    if launch == 0 {
        format!("{agent}-{}", std::process::id())
    } else {
        format!("{agent}-{}r{launch}", std::process::id())
    }
}

/// The proxy container serving one run.
fn proxy_container(run: &str) -> String {
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
fn loop_prompt(turn: u32) -> String {
    format!("Loop iteration {turn}.\n\n{TASK_PROMPT}")
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
         Submitting is free: if the submission is invalid or scores less than your best score, \
         your best score is still what counts.",
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
/// nothing: attempts are recorded as they arrive, the score comes from that
/// log rather than from the branch, and the repository goes with the scorer at
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
/// commit the working one it already had. The best solving attempt is the
/// submission of record, so a push worse than one already scored changes
/// nothing, and a broken one is only an unsolved attempt. The single thing
/// this can do is recover work that would otherwise go with the home.
///
/// The sandbox is gone by now, so this runs in a container of its own on the
/// same home, entered through the bridge so the git host resolves and the
/// proxy socket carries the push. The sidecars are still up, since `play_run`
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
            &format!("{SCORE_HOST}:{SANDBOX_LOOPBACK}"),
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
/// scoring.
///
/// The proxy routes requests for the score host onto its socket.
fn start_score_server(run: &str, game: &str) -> std::io::Result<()> {
    let container = scorer_container(run);
    log::info!("starting the scoring server {container}");

    let task = read_only_mount(&format!("{GAMES_DIRECTORY}/{game}"), TASK_MOUNT)?;
    let readme = read_only_mount(
        &format!("{GAMES_DIRECTORY}/{TASK_INSTRUCTIONS}"),
        README_MOUNT,
    )?;

    process::run_and_assume_success(
        "docker",
        &[
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
            "--entrypoint",
            BASH,
            SCORER_IMAGE,
            SCORE_ENTRY,
            game,
        ],
    )?;

    await_socket(&container, SCORE_SOCKET_PATH)
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

/// What a run was started with, kept as `runs/<run>/run.json`.
///
/// Credentials are named but never written, so the file says which variables
/// the sandbox was given without carrying their values.
#[derive(serde::Serialize)]
struct Metadata<'a> {
    run: &'a str,
    agent: &'a str,
    model: &'a str,
    game: &'a str,
    thinking: Option<&'a str>,
    limit_seconds: u64,
    image: &'a str,
    /// What the agent was told to start on.
    prompt: &'a str,
    arguments: &'a [String],
    variables: Vec<&'a str>,
    started_seconds: u64,
}

/// Write down what the run was started with, before anything runs.
fn record_metadata(
    run: &str,
    command: &Agent,
    image: &str,
    prompt: &str,
    invocation: &crate::registry::Invocation,
) -> std::io::Result<()> {
    let directory = std::path::Path::new(RUN_DIRECTORY).join(run);
    std::fs::create_dir_all(&directory)?;

    let metadata = Metadata {
        run,
        agent: &command.name,
        model: &command.model,
        game: &command.game,
        thinking: command.thinking.as_deref(),
        limit_seconds: command.limit,
        image,
        prompt,
        arguments: &invocation.arguments,
        variables: invocation
            .variables
            .iter()
            .map(|(name, _)| name.as_str())
            .collect(),
        started_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(std::io::Error::other)?
            .as_secs(),
    };

    std::fs::write(
        directory.join(METADATA_FILE),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&metadata).map_err(std::io::Error::other)?
        ),
    )
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

/// Record which harness version the run was played with, and report it.
fn collect_version(run: &str, image: &str) -> std::io::Result<String> {
    let version = process::run_and_assume_success(
        "docker",
        &["run", "--rm", "--network", "none", image, VERSION_OPTION],
    )?;

    log::info!("harness version: {version}");

    std::fs::write(
        std::path::Path::new(RUN_DIRECTORY)
            .join(run)
            .join(VERSION_FILE),
        format!("{version}\n"),
    )?;

    Ok(version)
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
        return Ok(0);
    }

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

    Ok(0)
}

/// Run the named agent, one run per requested parallel slot, and report the
/// first failing status.
///
/// The images, the proxy hosts file and the egress network are prepared once,
/// so the runs cannot race each other rebuilding them. Every run then plays on
/// the image id resolved here, whatever gets rebuilt in the meantime.
pub fn run_agent(command: &Agent) -> std::io::Result<i32> {
    let agent = command.name.as_str();
    require_game(&command.game)?;

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

    let tag = format!("{AGENT_IMAGE_PREFIX}{agent}");
    build_image(&tag, &format!("{AGENT_CONTEXT}/{agent}"), force)?;
    let identity = image_id(&tag)?;

    build_scorer_image(force)?;

    std::fs::write(PROXY_HOSTS, crate::upstreams::nginx_map(&registry.hosts()))?;
    build_image(PROXY_IMAGE, PROXY_CONTEXT, force)?;
    ensure_egress_network()?;

    let base = run_base(agent);

    if command.parallel == 1 {
        return play_run(command, &identity, &base, invocation);
    }

    let outcomes: Vec<std::io::Result<i32>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (1..=command.parallel)
            .map(|index| {
                let run = format!("{base}-{index}");
                let invocation = invocation.clone();
                let identity = identity.as_str();
                scope.spawn(move || play_run(command, identity, &run, invocation))
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

/// Play one run on the resolved image against a proxy of its own, and report
/// its status.
///
/// The sidecars live and die with the run: `ava` starts them, runs the sandbox
/// against them and removes everything afterwards. Since the proxy serves one
/// run, its access log describes that run alone.
fn play_run(
    command: &Agent,
    identity: &str,
    run: &str,
    invocation: crate::registry::Invocation,
) -> std::io::Result<i32> {
    let staging = std::env::temp_dir().join(STAGING_DIRECTORY).join(run);
    let image = pin_image(identity, &command.name, run)?;

    log::info!(
        "run {run}: {} on {} playing {}",
        command.name,
        command.model,
        command.game
    );

    // A sidecar that fails to start must not skip the teardown below, or the
    // ones already started leak, so the whole startup lands in one status.
    let status = record_metadata(run, command, identity, TASK_PROMPT, &invocation)
        .and_then(|()| start_proxy(run))
        .and_then(|()| start_score_server(run, &command.game))
        .and_then(|()| prepare_agent_home(run, &image))
        .and_then(|()| run_sandbox(command, &image, run, &staging, invocation));

    let collected = collect_logs(run, &proxy_container(run), ACCESS_LOG, ERROR_LOG);
    drain_scorer(run);
    let attempts = collect_logs(run, &scorer_container(run), SCORE_LOG, SCORE_ERROR_LOG);
    let versioned = collect_version(run, &image);
    remove_sidecars(run);

    let scored = match (&status, &collected, &attempts, &versioned) {
        (Ok(_), Ok(()), Ok(()), Ok(version)) => score_run(run, command, version),
        _ => Ok(()),
    };

    unpin_image(&image);

    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }

    let code = status?;
    collected?;
    attempts?;
    versioned?;
    scored?;
    Ok(code)
}

/// Fail before anything is built when the game or its task folder is missing.
fn require_game(game: &str) -> std::io::Result<()> {
    if ava_game::find(game).is_none() {
        return Err(crate::registry::unknown(
            game,
            "game",
            ava_game::GAMES.iter().map(|game| game.name()),
        ));
    }

    let task = std::path::Path::new(GAMES_DIRECTORY).join(game);
    if !task.is_dir() {
        return Err(std::io::Error::other(format!(
            "the task folder {} does not exist",
            task.display()
        )));
    }

    Ok(())
}

/// Aggregate the run's logs into the report kept as `runs/<run>/score.json`.
///
/// Every submission was scored by the scoring server the moment it was
/// posted, so this only reads logs and nothing executes here.
fn score_run(run: &str, command: &Agent, harness_version: &str) -> std::io::Result<()> {
    let directory = std::path::Path::new(RUN_DIRECTORY).join(run);
    log::info!("aggregating the {run} logs into the report");

    let report = ava_scorer::score::report(
        &ava_scorer::score::Score {
            game: None,
            metrics: Some(directory.join(ACCESS_LOG).display().to_string()),
            attempts: Some(directory.join(SCORE_LOG).display().to_string()),
        },
        Some(ava_scorer::score::Run {
            harness: &command.name,
            harness_version,
            model: &command.model,
            game: &command.game,
            thinking: command.thinking.as_deref(),
            limit_seconds: command.limit,
        }),
    )?;

    println!("{report}");
    std::fs::write(directory.join(SCORE_FILE), format!("{report}\n"))
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
    let container = sandbox_container(run);
    let registry = crate::registry::load()?;
    let mut phase = Phase {
        limit: command.limit,
        started: std::time::Instant::now(),
        run_limit: command.limit,
        last_call: false,
        turn: 1,
        monitor: std::sync::Arc::new(crate::monitor::Monitor::new()),
    };

    let mut turn = invocation;

    loop {
        let entered = std::time::Instant::now();
        let ending = start_sandbox(command, image, run, staging, &turn, &phase, &container)?;

        let code = match ending {
            Ending::Done(code) => return Ok(code),
            Ending::OutOfTime(_) => {
                return last_call(command, image, run, staging, &container, &phase);
            }
            Ending::TurnOver(code) => code,
        };

        let Some(left) = phase.remaining() else {
            log::info!("{run}: the clock ran out as the turn ended, going to the last call");
            return last_call(command, image, run, staging, &container, &phase);
        };

        // A harness failing at startup would otherwise spin through the clock.
        let spent = entered.elapsed().as_secs();
        if spent < TURN_RETRY_SECONDS {
            log::warn!(
                "{run}: turn {} ended after {spent} seconds with {code}, waiting to start the next",
                phase.turn
            );
            std::thread::sleep(std::time::Duration::from_secs(TURN_RETRY_SECONDS));
        }

        phase.turn += 1;
        phase.limit = left;
        turn = registry.invocation(
            &command.name,
            &command.model,
            &loop_prompt(phase.turn),
            command.thinking.as_deref(),
            crate::registry::Start::Resume,
        )?;
    }
}

/// Start the agent one final turn, prompted to submit instead of to carry on.
///
/// The home is a volume, so the harness session and the workspace are both
/// still there and this turn continues the conversation like any other. The
/// only thing that sets it apart is the prompt and that no turn follows it.
fn last_call(
    command: &Agent,
    image: &str,
    run: &str,
    staging: &std::path::Path,
    container: &str,
    task: &Phase,
) -> std::io::Result<i32> {
    let prompt = last_call_prompt();
    let invocation = crate::registry::load()?.invocation(
        &command.name,
        &command.model,
        &prompt,
        command.thinking.as_deref(),
        crate::registry::Start::Resume,
    )?;

    log::info!("{run}: starting the agent for the last call");

    let phase = Phase {
        limit: LAST_CALL_SECONDS,
        started: task.started,
        run_limit: task.run_limit,
        last_call: true,
        turn: task.turn + 1,
        monitor: task.monitor.clone(),
    };

    let ending = start_sandbox(command, image, run, staging, &invocation, &phase, container);

    // Whatever the agent left is worth submitting even when the last call
    // itself never got going.
    last_chance(run, image);

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

/// Start one sandbox on `image`, wait for it and report why it stopped.
fn start_sandbox(
    command: &Agent,
    image: &str,
    run: &str,
    staging: &std::path::Path,
    invocation: &crate::registry::Invocation,
    phase: &Phase,
    container: &str,
) -> std::io::Result<Ending> {
    let agent = command.name.as_str();

    let mut sandbox = std::process::Command::new("docker");
    sandbox.args([
        "run",
        "--rm",
        "--name",
        container,
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
        &format!("{}:{SOCKET_DIRECTORY}{READ_ONLY}", socket_volume(run)),
    ]);

    sandbox.args(home_mounts(run));

    sandbox.args(["--hostname", agent]);
    sandbox.args(["--add-host", &format!("{agent}:{SANDBOX_LOOPBACK}")]);

    for host in &invocation.hosts {
        sandbox.args(["--add-host", &format!("{host}:{SANDBOX_LOOPBACK}")]);
    }

    sandbox.args(["--add-host", &format!("{SCORE_HOST}:{SANDBOX_LOOPBACK}")]);

    for (path, contents) in &invocation.files {
        sandbox.args(["--volume", &staged_mount(staging, path, contents)?]);
    }

    for (name, value) in &invocation.variables {
        sandbox.args(["--env", name]);
        sandbox.env(name, value);
    }

    // Every turn starts a container of the same name, and `--rm` frees that
    // name a moment after the client exits rather than before, so the name is
    // cleared here instead of raced for.
    remove_container(container);

    log::info!("starting the sandbox {container} from {image}");
    log::debug!("{agent} arguments: {:?}", invocation.arguments);

    sandbox.stdout(std::process::Stdio::piped());
    sandbox.stderr(std::process::Stdio::piped());

    // The client gets its own process group, so signals aimed at the terminal
    // cannot kill it and the run ends only when ava ends it.
    std::os::unix::process::CommandExt::process_group(&mut sandbox, 0);

    let mut child = sandbox.arg(image).args(&invocation.arguments).spawn()?;

    phase.monitor.restart();
    let readers = record_output(&mut child, run, &phase.monitor)?;
    let ending = await_sandbox(child, container, run, phase)?;

    for reader in readers {
        reader.join().expect("the reader threads do not panic")?;
    }

    Ok(ending)
}

/// The file keeping what the agent printed.
///
/// Appended rather than truncated, so the last call adds to the console of the
/// run instead of replacing what the task start wrote.
fn agent_log(run: &str) -> std::io::Result<std::sync::Arc<std::sync::Mutex<std::fs::File>>> {
    let directory = std::path::Path::new(RUN_DIRECTORY).join(run);
    std::fs::create_dir_all(&directory)?;

    Ok(std::sync::Arc::new(std::sync::Mutex::new(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join(AGENT_LOG))?,
    )))
}

/// Keep what the agent printed in `runs/<run>/agent.log`.
///
/// The terminal gets the periodic status log instead of the console. Only the
/// stdout stream carrying the harness events is scanned for repeated lines.
fn record_output(
    child: &mut std::process::Child,
    run: &str,
    monitor: &std::sync::Arc<crate::monitor::Monitor>,
) -> std::io::Result<Vec<std::thread::JoinHandle<std::io::Result<()>>>> {
    let stdout = child.stdout.take().expect("the sandbox output is piped");
    let stderr = child.stderr.take().expect("the sandbox output is piped");

    let log = agent_log(run)?;

    Ok(vec![
        record(stdout, log.clone(), monitor.clone(), true),
        record(stderr, log, monitor.clone(), false),
    ])
}

/// Copy `source` into `log` until it ends, reporting every chunk to the
/// monitor.
fn record(
    mut source: impl std::io::Read + Send + 'static,
    log: std::sync::Arc<std::sync::Mutex<std::fs::File>>,
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
            std::io::Write::write_all(
                &mut *log.lock().expect("the agent log is not poisoned"),
                &buffer[..read],
            )?;
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
    /// The seconds the run as a whole was given.
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
    /// The seconds left of the run, or nothing once they are spent.
    fn remaining(&self) -> Option<u64> {
        self.run_limit
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
fn record_heartbeat(run: &str, phase: &Phase) {
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
                .join(run)
                .join(MONITOR_FILE),
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
    container: &str,
    run: &str,
    phase: &Phase,
) -> std::io::Result<Ending> {
    let monitor = phase.monitor.as_ref();
    let entered = std::time::Instant::now();
    let deadline = entered + std::time::Duration::from_secs(phase.limit);
    let mut next_status = entered + STATUS_INTERVAL;
    let mut warned_silence = std::time::Duration::ZERO;
    let mut warned_looping = false;
    let mut out_of_time = false;
    let scorer = scorer_container(run);
    log::info!(
        "{run}: turn {} of the agent has {} seconds",
        phase.turn,
        phase.limit
    );
    record_heartbeat(run, phase);

    loop {
        let exited = client.try_wait()?;

        if crate::interrupt::interrupted() {
            log::warn!("the run was interrupted, killing {container}");
        } else if let Some(status) = exited {
            log::info!(
                "{run}: turn {} of the agent exited with {status}",
                phase.turn
            );
            return Ok(Ending::TurnOver(status.code().unwrap_or(1)));
        } else if exists(&["exec", &scorer, "test", "-f", DONE_MARKER])? {
            log::info!("{run}: the agent reported itself done, stopping {container}");
        } else if std::time::Instant::now() >= deadline {
            log::warn!(
                "{run}: the agent ran out of time after {} seconds, stopping it",
                phase.limit
            );
            out_of_time = true;
        } else {
            let silence = monitor.silent_for();

            if silence < SILENCE_WARNING {
                warned_silence = std::time::Duration::ZERO;
            } else if silence - warned_silence >= SILENCE_WARNING {
                log::warn!(
                    "{run}: the agent printed nothing for {} seconds, maybe a thinking request runs long",
                    silence.as_secs()
                );
                warned_silence = silence;
            }

            if !warned_looping && monitor.doom_looping() {
                log::warn!(
                    "{run}: the agent repeated one output line {} times in a row, it may be stuck in a loop",
                    crate::monitor::REPEATED_LINE_THRESHOLD
                );
                warned_looping = true;
            }

            if std::time::Instant::now() >= next_status {
                log::info!(
                    "{run}: turn {}, {}s of {}s, {} KiB from the agent, the last output {}s ago",
                    phase.turn,
                    phase.started.elapsed().as_secs(),
                    phase.run_limit,
                    monitor.output_bytes() / KIBIBYTE,
                    silence.as_secs()
                );
                record_heartbeat(run, phase);
                next_status += STATUS_INTERVAL;
            }

            std::thread::sleep(CLOCK_INTERVAL);
            continue;
        }

        let _ = std::process::Command::new("docker")
            .args(["kill", container])
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
