mod cli;
mod commands;
mod config;
mod git;
mod manifest;
mod operations;
mod process;
mod workspace;
mod yaml;

fn main() {
    if let Err(error) = process::init() {
        eprintln!("Failed to install signal handlers: {error}");
        std::process::exit(1);
    }
    let tokens: Vec<String> = std::env::args().skip(1).collect();
    if std::env::var("CLOWDER_COMPLETE").as_deref() == Ok("probe") {
        println!("clowder-native-completion-v1");
        return;
    }
    if std::env::var("CLOWDER_COMPLETE").as_deref() == Ok("1") {
        for candidate in cli::complete(&tokens) {
            println!("{candidate}");
        }
        return;
    }
    match cli::parse(tokens) {
        Ok(Some(args)) => {
            if let Err(error) = commands::run(&args).and_then(|()| process::check()) {
                eprintln!("{error:#}");
                std::process::exit(process::exit_code().unwrap_or_else(|| {
                    error
                        .downcast_ref::<commands::ExitCode>()
                        .map_or(1, |e| e.0)
                }));
            }
        }
        Ok(None) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}
