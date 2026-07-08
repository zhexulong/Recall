use anyhow::Result;
use clap::Parser;
use recall_reflect::manifest;

#[derive(Parser)]
#[command(name = "recall-reflect", version, about = "Reflect on Recall session history")]
struct Cli {
    #[arg(long = "recall-extension-manifest", hide = true)]
    recall_extension_manifest: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.recall_extension_manifest {
        println!("{}", manifest::manifest_json());
        return Ok(());
    }
    Ok(())
}
