use anyhow::Result;
use clap::{Parser, ValueEnum};

use recall_reflect::manifest;
use recall_reflect::protocol::{RecallClient, ReflectArgs};
use recall_reflect::render::render_text;
use recall_reflect::report::build_reflect_report;

#[derive(Parser)]
#[command(name = "recall-reflect", version, about = "Reflect on Recall session history")]
struct Cli {
    #[arg(long = "recall-extension-manifest", hide = true)]
    recall_extension_manifest: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    time: Option<String>,
    #[arg(long)]
    project: Option<String>,
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    sync: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.recall_extension_manifest {
        println!("{}", manifest::manifest_json());
        return Ok(());
    }

    let args =
        ReflectArgs { source: cli.source, time: cli.time, project: cli.project, repo: cli.repo };

    let client = RecallClient::from_env();
    if cli.sync {
        client.sync(args.source.as_deref())?;
    }

    let filters = args.filters();
    let sessions = client.export_sessions(&args)?;
    let report = build_reflect_report(sessions, &filters);

    match cli.format {
        OutputFormat::Text => print!("{}", render_text(&report)),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }

    Ok(())
}
