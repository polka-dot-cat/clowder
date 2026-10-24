mod cli;
mod commands;
mod config;
mod git;
mod manifest;
mod operations;
mod workspace;
mod yaml;

fn main() {
    match cli::parse(std::env::args().skip(1).collect()) {
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
