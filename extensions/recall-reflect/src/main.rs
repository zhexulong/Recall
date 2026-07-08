use std::process::Command;

use anyhow::{Context, Result, bail};
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

    let mut args =
        ReflectArgs { source: cli.source, time: cli.time, project: cli.project, repo: cli.repo };
    apply_default_scope(&mut args)?;

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

fn apply_default_scope(args: &mut ReflectArgs) -> Result<()> {
    if args.project.is_some() || args.repo.is_some() {
        return Ok(());
    }

    args.project = Some(current_git_root()?);
    Ok(())
}

fn current_git_root() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to resolve the current git repository; pass --project or --repo")?;

    if !output.status.success() {
        bail!("recall-reflect needs a scope outside a git worktree; pass --project or --repo");
    }

    let root = String::from_utf8(output.stdout)
        .context("git repository root was not valid UTF-8; pass --project or --repo")?
        .trim()
        .to_string();

    if root.is_empty() {
        bail!("git did not return a repository root; pass --project or --repo");
    }

    Ok(root)
}
