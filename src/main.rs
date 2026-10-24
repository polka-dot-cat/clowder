mod cli;
mod commands;
mod config;
mod git;
mod manifest;
mod operations;
mod workspace;
mod yaml;

fn main() {
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
            if let Err(error) = commands::run(&args) {
                eprintln!("{error:#}");
                std::process::exit(
                    error
                        .downcast_ref::<commands::ExitCode>()
                        .map_or(1, |e| e.0),
                );
            }
        }
        Ok(None) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}
