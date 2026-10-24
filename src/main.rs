mod aggregate;
mod bench;
mod bruteforce;
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
        #[arg(long)]
        context: Option<PathBuf>,
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
    BruteforceBench {
        #[arg(long, default_value = "eval/historical")]
        historical: PathBuf,
        #[arg(long, default_value = "z-ai/glm-5.3-flash")]
        model: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        full_rules: Option<PathBuf>,
        #[arg(long = "pr")]
        prs: Vec<u64>,
        #[arg(long, default_value = "both")]
        order: String,
        #[arg(long, default_value_t = 5)]
        review_count: usize,
        #[arg(long, default_value_t = 3)]
        final_reducers: usize,
        #[arg(long)]
        skip_prefix_reducers: bool,
        #[arg(long)]
        include_reasoning: bool,
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
            context,
            models,
            temperature,
            samples,
        } => review::run(&diff, context.as_deref(), &models, temperature, samples).await,
        Command::Preflight { diff } => review::preflight(&diff),
        Command::Bench {
            fixtures,
            models,
            temperature,
            output,
            samples,
        } => bench::run(&fixtures, &models, temperature, output.as_deref(), samples).await,
        Command::BruteforceBench {
            historical,
            model,
            output,
            full_rules,
            prs,
            order,
            review_count,
            final_reducers,
            skip_prefix_reducers,
            include_reasoning,
        } => {
            bruteforce::run(
                &historical,
                &model,
                &output,
                &prs,
                full_rules.as_deref(),
                &order,
                bruteforce::BenchPlan {
                    review_count,
                    final_reducers,
                    include_prefix_reducers: !skip_prefix_reducers,
                    include_reasoning,
                },
            )
            .await
        }
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
