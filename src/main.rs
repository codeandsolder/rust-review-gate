mod aggregate;
mod bench;
mod diff;
mod github;
mod model;
mod review;
mod rules;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Review {
        #[arg(long)]
        diff: PathBuf,
        #[arg(long = "model", required = true)]
        models: Vec<String>,
        #[arg(long, default_value_t = 0.10)]
        temperature: f32,
        #[arg(long, default_value_t = 1)]
        samples: usize,
    },
    Preflight {
        #[arg(long)]
        diff: PathBuf,
    },
    Bench {
        #[arg(long, default_value = "eval/fixtures")]
        fixtures: PathBuf,
        #[arg(long = "model", required = true)]
        models: Vec<String>,
        #[arg(long, default_value_t = 0.0)]
        temperature: f32,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, default_value_t = 1)]
        samples: usize,
    },
    Github {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        pull: u64,
        #[arg(long = "model", required = true)]
        models: Vec<String>,
        #[arg(long, default_value_t = 2)]
        min_votes: usize,
        #[arg(long, default_value_t = 0.0)]
        temperature: f32,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value_t = 1)]
        samples: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Review {
            diff,
            models,
            temperature,
            samples,
        } => review::run(&diff, &models, temperature, samples).await,
        Command::Preflight { diff } => review::preflight(&diff),
        Command::Bench {
            fixtures,
            models,
            temperature,
            output,
            samples,
        } => bench::run(&fixtures, &models, temperature, output.as_deref(), samples).await,
        Command::Github {
            repo,
            pull,
            models,
            min_votes,
            temperature,
            dry_run,
            samples,
        } => {
            github::run(
                &repo,
                pull,
                &models,
                min_votes,
                temperature,
                dry_run,
                samples,
            )
            .await
        }
    }
}
