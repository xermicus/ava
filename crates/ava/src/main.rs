mod arguments;
mod environment;

/// The level logged unless `RUST_LOG` asks for another one.
const DEFAULT_LOG_LEVEL: &str = "info";

fn main() {
    if let Err(error) = environment::load() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }

    env_logger::Builder::new()
        .parse_env(env_logger::Env::default().default_filter_or(DEFAULT_LOG_LEVEL))
        .init();

    ava_run::interrupt::install();

    let command = arguments::Parser::parse(std::env::args().skip(1));

    let outcome = match command {
        Some(arguments::SubCommand::Agent(ref agent)) => ava_run::docker::run_agent(agent),
        Some(arguments::SubCommand::Image(ref image)) => ava_run::docker::build_images(image),
        Some(arguments::SubCommand::Analyze(ref command)) => ava_run::docker::analyze(command),
        Some(arguments::SubCommand::Score(ref command)) => ava_scorer::score::run(command),
        Some(arguments::SubCommand::Serve(ref command)) => ava_web::serve::run(command),
        Some(arguments::SubCommand::Remote(ref command)) => ava_scorer::remote::run(command),
        Some(arguments::SubCommand::Upstreams(ref command)) => ava_run::upstreams::run(command),
        Some(arguments::SubCommand::Tournament(ref command)) => ava_run::tournament::run(command),
        None => arguments::help(),
    };

    match outcome {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
