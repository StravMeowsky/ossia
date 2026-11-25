use clap::{Parser, Subcommand};
use ossia_omr::Config;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ossia-omr", version, about = "Remote Audiveris OMR service")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Serve {
        #[arg(long)]
        home: Option<PathBuf>,
        #[arg(long)]
        listen: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let result = match args.command {
        Command::Serve { home, listen } => {
            let config = Config::discover(home, listen);
            match config {
                Ok(config) => ossia_omr::serve(config).await,
                Err(error) => Err(error),
            }
        }
    };
    if let Err(error) = result {
        println!("ossia-omr: {}", error.message);
        std::process::exit(1);
    }
}
