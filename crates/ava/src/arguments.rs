//! The AvA CLI argument data structures and parser implementation.
//!
//! The command payloads live next to their implementations in the `ava-run`,
//! `ava-scorer` and `ava-web` crates; this module only knows the flags, the
//! help texts and the validation filling them.

/// General arguments applicable to all sub commands.
pub(crate) struct Arguments;

impl Arguments {
    const LIST_MODELS: &str = "list-models";
    const LIST_AGENTS: &str = "list-agents";
    const USAGE: &str = "usage";

    /// Prints the help message to stdout.
    fn help() {
        arg_help_str(
            &format!("--{}", Self::LIST_MODELS),
            "print the known models and exit",
        );
        arg_help_str(
            &format!("--{}", Self::LIST_AGENTS),
            "print the known agents and exit",
        );
        arg_help_str(
            &format!("--{}", Self::USAGE),
            "print the limits of every backend and the recorded usage, then exit",
        );
    }
}

/// The overarching sub command definitions.
#[derive(Debug)]
pub(crate) enum SubCommand {
    Agent(ava_run::docker::Agent),
    Image(ava_run::docker::Image),
    Score(ava_scorer::score::Score),
    Serve(ava_web::serve::Serve),
    Remote(ava_scorer::remote::Remote),
    Upstreams(ava_run::upstreams::Upstreams),
}

/// The command line of the agent sandbox command.
struct AgentCli;

impl AgentCli {
    const NAME: &str = "agent";
    const DESCRIPTION: &str = "run an agent in a sandbox on the internal network";

    const AGENT_SHORT: char = 'a';
    const MODEL_SHORT: char = 'm';
    const GAME_SHORT: char = 'g';
    const TIME_LIMIT_SHORT: char = 't';
    const PARALLEL_SHORT: char = 'j';
    const THINKING_SHORT: char = 'e';
    const FORCE_BUILD_LONG: &str = "force-build-images";

    fn help() {
        command_help(Self::NAME, Self::DESCRIPTION);
        arg_help_chr(Self::AGENT_SHORT, "the agent to run");
        arg_help_chr(Self::MODEL_SHORT, "the model name to use");
        arg_help_chr(Self::GAME_SHORT, "the game to play and score");
        arg_help_chr(
            Self::TIME_LIMIT_SHORT,
            &format!(
                "the seconds the agent is given including its {} second last call, {} by default",
                ava_run::docker::LAST_CALL_SECONDS,
                ava_run::docker::Agent::DEFAULT_LIMIT_SECONDS
            ),
        );
        arg_help_chr(
            Self::PARALLEL_SHORT,
            &format!(
                "the runs started in parallel, {} by default",
                ava_run::docker::Agent::DEFAULT_PARALLEL_RUNS
            ),
        );
        arg_help_chr(
            Self::THINKING_SHORT,
            &format!(
                "how much thinking to ask for: {}",
                ava_run::registry::THINKING_LEVELS.join(", ")
            ),
        );
        arg_help_str(
            &format!("--{}", Self::FORCE_BUILD_LONG),
            "rebuild the docker images instead of reusing the built ones",
        );
    }

    /// Exit(1) unless every argument this command requires was given.
    fn require_arguments(command: &ava_run::docker::Agent) {
        for (value, flag, subject) in [
            (&command.name, Self::AGENT_SHORT, "agent"),
            (&command.model, Self::MODEL_SHORT, "model"),
            (&command.game, Self::GAME_SHORT, "game"),
        ] {
            if value.is_empty() {
                fail(&format!("no {subject} given, pass one with -{flag}"));
            }
        }
    }
}

/// The command line of the image building command.
struct ImageCli;

impl ImageCli {
    const NAME: &str = "image";
    const DESCRIPTION: &str = "rebuild docker images, all of them without arguments";

    const AGENT_SHORT: char = 'a';
    const PROXY_SHORT: char = 'p';
    const SCORER_SHORT: char = 's';

    fn help() {
        command_help(Self::NAME, Self::DESCRIPTION);
        arg_help_chr(Self::AGENT_SHORT, "the harness to build for");
        arg_help_chr(Self::PROXY_SHORT, "build the proxy image");
        arg_help_chr(Self::SCORER_SHORT, "build the scorer image");
    }
}

/// The command line of the submission scoring command.
struct ScoreCli;

impl ScoreCli {
    const NAME: &str = "score";
    const DESCRIPTION: &str = "score a submission and aggregate the metrics of a run";

    const METRICS_LONG: &str = "metrics";
    const GAME_LONG: &str = "game";
    const ATTEMPTS_LONG: &str = "attempts";

    fn help() {
        command_help(Self::NAME, Self::DESCRIPTION);
        arg_help_str(
            &format!("--{}", Self::METRICS_LONG),
            "the proxy access log to aggregate",
        );
        arg_help_str(
            &format!("--{}", Self::GAME_LONG),
            "the game scoring the submission",
        );
        arg_help_str(
            &format!("--{}", Self::ATTEMPTS_LONG),
            "the live scoring log to aggregate",
        );
    }

    /// Exit(1) unless at least one report was requested.
    fn require_arguments(command: &ava_scorer::score::Score) {
        if command.metrics.is_none() && command.game.is_none() && command.attempts.is_none() {
            fail(&format!(
                "nothing to score, pass --{}, --{} or --{}",
                Self::GAME_LONG,
                Self::METRICS_LONG,
                Self::ATTEMPTS_LONG
            ));
        }
    }
}

/// The command line of the web interface command.
struct ServeCli;

impl ServeCli {
    const NAME: &str = "serve";
    const DESCRIPTION: &str = "serve the web interface on localhost";

    const PORT_SHORT: char = ImageCli::PROXY_SHORT;

    fn help() {
        command_help(Self::NAME, Self::DESCRIPTION);
        arg_help_chr(
            Self::PORT_SHORT,
            &format!(
                "the port to bind, {} by default",
                ava_web::serve::DEFAULT_PORT
            ),
        );
    }
}

/// The command line of the git remote command.
struct RemoteCli;

impl RemoteCli {
    const NAME: &str = "remote";
    const DESCRIPTION: &str = "serve the git remote and the scorer of a run on the score socket";

    const SOCKET_LONG: &str = "socket";
    const ROOT_LONG: &str = "root";

    fn help() {
        command_help(Self::NAME, Self::DESCRIPTION);
        arg_help_str(
            &format!("--{}", Self::SOCKET_LONG),
            "the socket to listen on instead of the score socket",
        );
        arg_help_str(
            &format!("--{}", Self::ROOT_LONG),
            "the directory holding the repository instead of the agent home",
        );
    }
}

/// The command line of the upstream endpoint listing command.
struct UpstreamsCli;

impl UpstreamsCli {
    const NAME: &str = "upstreams";
    const DESCRIPTION: &str = "print the allowed LLM endpoints, one host per line";

    const NGINX_MAP_SHORT: char = 'n';

    fn help() {
        command_help(Self::NAME, Self::DESCRIPTION);
        arg_help_chr(Self::NGINX_MAP_SHORT, "print an nginx map block instead");
    }
}

/// The argument parser.
#[derive(Default)]
pub(crate) struct Parser {
    command: Option<SubCommand>,
}

impl Parser {
    /// Parse the given `args` into a runnable command and exit(1) on failure.
    pub(crate) fn parse(mut args: impl Iterator<Item = String>) -> Option<SubCommand> {
        let mut parser = Self::default();

        while let Some(next) = args.next() {
            match next.as_str() {
                // Sub commands
                AgentCli::NAME => {
                    parser.command = Some(SubCommand::Agent(Default::default()));
                }
                ImageCli::NAME => {
                    parser.command = Some(SubCommand::Image(Default::default()));
                }
                ScoreCli::NAME => {
                    parser.command = Some(SubCommand::Score(Default::default()));
                }
                ServeCli::NAME => {
                    parser.command = Some(SubCommand::Serve(Default::default()));
                }
                RemoteCli::NAME => {
                    parser.command = Some(SubCommand::Remote(Default::default()));
                }
                UpstreamsCli::NAME => {
                    parser.command = Some(SubCommand::Upstreams(Default::default()));
                }

                _ if next.starts_with("--") => {
                    parser.long(&mut args, next.trim_start_matches("--"));
                }

                _ if next.starts_with("-") => {
                    parser.short(&mut args, next.trim_start_matches("-"));
                }

                _ => bail(&next, "unknown argument"),
            }
        }

        match parser.command {
            Some(SubCommand::Agent(ref command)) => AgentCli::require_arguments(command),
            Some(SubCommand::Score(ref command)) => ScoreCli::require_arguments(command),
            Some(SubCommand::Image(_))
            | Some(SubCommand::Serve(_))
            | Some(SubCommand::Remote(_))
            | Some(SubCommand::Upstreams(_))
            | None => {}
        }

        parser.command
    }

    fn value(
        args: &mut impl Iterator<Item = String>,
        chars: &mut std::iter::Peekable<impl Iterator<Item = char>>,
        flag: &str,
        reason: &str,
    ) -> String {
        let value: String = chars.collect();
        if !value.is_empty() {
            return value;
        }
        args.next().unwrap_or_else(|| bail(flag, reason))
    }

    fn short(&mut self, args: &mut impl Iterator<Item = String>, next: &str) {
        let mut chars = next.chars().peekable();
        while let Some(char) = chars.next() {
            let mut buf = [0; 4];
            let flag = char.encode_utf8(&mut buf);
            match char {
                // Agent and image
                AgentCli::AGENT_SHORT => {
                    let name = Self::value(args, &mut chars, flag, "missing agent name");
                    match self.command {
                        Some(SubCommand::Agent(ref mut command)) => command.name = name,
                        Some(SubCommand::Image(ref mut command)) => command.agent = name,
                        _ => bail(
                            flag,
                            &format!(
                                "only valid in the {} or {} subcommands",
                                AgentCli::NAME,
                                ImageCli::NAME
                            ),
                        ),
                    }
                    break;
                }
                AgentCli::MODEL_SHORT => {
                    let model = Self::value(args, &mut chars, flag, "missing model name");
                    let Some(SubCommand::Agent(ref mut command)) = self.command else {
                        bail(
                            flag,
                            &format!("only valid in the {} subcommand", AgentCli::NAME),
                        );
                    };
                    command.model = model;
                    break;
                }
                AgentCli::GAME_SHORT => {
                    let game = Self::value(args, &mut chars, flag, "missing game name");
                    let Some(SubCommand::Agent(ref mut command)) = self.command else {
                        bail(
                            flag,
                            &format!("only valid in the {} subcommand", AgentCli::NAME),
                        );
                    };
                    command.game = game;
                    break;
                }
                AgentCli::TIME_LIMIT_SHORT => {
                    let seconds = Self::value(args, &mut chars, flag, "missing time limit");
                    let limit: u64 = seconds
                        .parse()
                        .unwrap_or_else(|_| bail(flag, "the limit is a number of seconds"));
                    if limit < ava_run::docker::LAST_CALL_SECONDS {
                        bail(
                            flag,
                            &format!(
                                "the limit pays for the last call, so it is at least {} seconds",
                                ava_run::docker::LAST_CALL_SECONDS
                            ),
                        );
                    }
                    let Some(SubCommand::Agent(ref mut command)) = self.command else {
                        bail(
                            flag,
                            &format!("only valid in the {} subcommand", AgentCli::NAME),
                        );
                    };
                    command.limit = limit;
                    break;
                }
                AgentCli::PARALLEL_SHORT => {
                    let runs = Self::value(args, &mut chars, flag, "missing run count");
                    let parallel: u64 = runs
                        .parse()
                        .unwrap_or_else(|_| bail(flag, "the run count is a number"));
                    if parallel == 0 {
                        bail(flag, "at least one run must be started");
                    }
                    let Some(SubCommand::Agent(ref mut command)) = self.command else {
                        bail(
                            flag,
                            &format!("only valid in the {} subcommand", AgentCli::NAME),
                        );
                    };
                    command.parallel = parallel;
                    break;
                }
                AgentCli::THINKING_SHORT => {
                    let level = Self::value(args, &mut chars, flag, "missing thinking level");
                    if !ava_run::registry::THINKING_LEVELS.contains(&level.as_str()) {
                        bail(
                            flag,
                            &format!(
                                "unknown thinking level `{level}`, known are: {}",
                                ava_run::registry::THINKING_LEVELS.join(", ")
                            ),
                        );
                    }
                    let Some(SubCommand::Agent(ref mut command)) = self.command else {
                        bail(
                            flag,
                            &format!("only valid in the {} subcommand", AgentCli::NAME),
                        );
                    };
                    command.thinking = Some(level);
                    break;
                }
                // Image and serve
                ImageCli::PROXY_SHORT => match self.command {
                    Some(SubCommand::Image(ref mut command)) => command.proxy = true,
                    Some(SubCommand::Serve(ref mut command)) => {
                        let port = Self::value(args, &mut chars, flag, "missing port");
                        command.port = port
                            .parse()
                            .unwrap_or_else(|_| bail(flag, "the port is a number"));
                        break;
                    }
                    _ => bail(
                        flag,
                        &format!(
                            "only valid in the {} or {} subcommands",
                            ImageCli::NAME,
                            ServeCli::NAME
                        ),
                    ),
                },
                ImageCli::SCORER_SHORT => {
                    let Some(SubCommand::Image(ref mut command)) = self.command else {
                        bail(
                            flag,
                            &format!("only valid in the {} subcommand", ImageCli::NAME),
                        );
                    };
                    command.scorer = true;
                }

                // Upstreams
                UpstreamsCli::NGINX_MAP_SHORT => {
                    let Some(SubCommand::Upstreams(ref mut command)) = self.command else {
                        bail(
                            flag,
                            &format!("only valid in the {} subcommand", UpstreamsCli::NAME),
                        );
                    };
                    command.nginx_map = true;
                }

                _ => bail(flag, "unknown argument"),
            }
        }
    }

    fn long(&mut self, args: &mut impl Iterator<Item = String>, next: &str) {
        match next {
            Arguments::LIST_MODELS => ava_run::registry::list_models(),
            Arguments::LIST_AGENTS => ava_run::registry::list_agents(),
            Arguments::USAGE => ava_run::usage::print(),
            AgentCli::FORCE_BUILD_LONG => {
                let Some(SubCommand::Agent(ref mut command)) = self.command else {
                    bail(
                        next,
                        &format!("only valid in the {} subcommand", AgentCli::NAME),
                    );
                };
                command.force_build_images = true;
            }
            ScoreCli::METRICS_LONG => {
                let log = Self::long_value(args, next, "missing log file");
                let Some(SubCommand::Score(ref mut command)) = self.command else {
                    bail(
                        next,
                        &format!("only valid in the {} subcommand", ScoreCli::NAME),
                    );
                };
                command.metrics = Some(log);
            }
            ScoreCli::GAME_LONG => {
                let game = Self::long_value(args, next, "missing game name");
                let Some(SubCommand::Score(ref mut command)) = self.command else {
                    bail(
                        next,
                        &format!("only valid in the {} subcommand", ScoreCli::NAME),
                    );
                };
                command.game = Some(game);
            }
            ScoreCli::ATTEMPTS_LONG => {
                let log = Self::long_value(args, next, "missing log file");
                let Some(SubCommand::Score(ref mut command)) = self.command else {
                    bail(
                        next,
                        &format!("only valid in the {} subcommand", ScoreCli::NAME),
                    );
                };
                command.attempts = Some(log);
            }
            RemoteCli::SOCKET_LONG => {
                let socket = Self::long_value(args, next, "missing socket path");
                let Some(SubCommand::Remote(ref mut command)) = self.command else {
                    bail(
                        next,
                        &format!("only valid in the {} subcommand", RemoteCli::NAME),
                    );
                };
                command.socket = Some(socket);
            }
            RemoteCli::ROOT_LONG => {
                let root = Self::long_value(args, next, "missing root directory");
                let Some(SubCommand::Remote(ref mut command)) = self.command else {
                    bail(
                        next,
                        &format!("only valid in the {} subcommand", RemoteCli::NAME),
                    );
                };
                command.root = Some(root);
            }
            _ => bail(next, "unknown argument"),
        }
    }

    /// The value following a long flag, or exit(1) if none was given.
    fn long_value(args: &mut impl Iterator<Item = String>, flag: &str, reason: &str) -> String {
        args.next().unwrap_or_else(|| bail(flag, reason))
    }
}

fn bail(token: &str, reason: &str) -> ! {
    fail(&format!("`{token}`: {reason}"));
}

/// Print `reason`, then the help, and exit(1).
pub(crate) fn fail(reason: &str) -> ! {
    eprintln!("error: {reason}\n");
    help();
}

/// Print the help and exit(1) to STDERR.
pub(crate) fn help() -> ! {
    eprintln!("Options applying to all sub-commands:\n");
    Arguments::help();

    eprintln!();

    eprintln!("Available sub-commands:\n");
    for print_command in [
        AgentCli::help,
        ImageCli::help,
        ScoreCli::help,
        ServeCli::help,
        RemoteCli::help,
        UpstreamsCli::help,
    ] {
        print_command();
        eprintln!();
    }

    std::process::exit(1);
}

const HELP_WIDTH: usize = 18;

fn command_help(name: &str, message: &str) {
    eprintln!("  {name:<HELP_WIDTH$}{message}");
}

fn arg_help_chr(argument: char, message: &str) {
    arg_help_str(&format!("-{argument}"), message);
}

fn arg_help_str(argument: &str, message: &str) {
    eprintln!("    {argument:<HELP_WIDTH$}  {message}");
}
