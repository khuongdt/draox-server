mod emitters;
mod model;
mod spec;

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "sdk-gen",
    about = "Generate client SDKs from an OpenAPI spec",
    version
)]
struct Cli {
    /// Path to the OpenAPI spec file (JSON or YAML)
    #[arg(short, long)]
    spec: PathBuf,

    /// Output directory
    #[arg(short, long, default_value = "sdk-out")]
    output: PathBuf,

    /// Base URL written into the generated clients
    #[arg(short, long, default_value = "https://api.draox-server.io")]
    base_url: String,

    /// Target languages (comma-separated: typescript,dart)
    #[arg(short, long, default_value = "typescript")]
    targets: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let spec = spec::load(&cli.spec).context("failed to load spec")?;
    let endpoints = model::extract_endpoints(&spec);

    if endpoints.is_empty() {
        eprintln!("warning: no endpoints found in spec");
    } else {
        println!("found {} endpoint(s)", endpoints.len());
    }

    std::fs::create_dir_all(&cli.output).context("cannot create output dir")?;

    for target in cli.targets.split(',').map(str::trim) {
        match target {
            "typescript" | "ts" => {
                let code = emitters::typescript::emit(&endpoints, &cli.base_url);
                let out = cli.output.join("client.ts");
                std::fs::write(&out, code).context("write typescript")?;
                println!("typescript → {}", out.display());
            }
            "dart" => {
                let code = emitters::dart::emit(&endpoints, &cli.base_url);
                let out = cli.output.join("client.dart");
                std::fs::write(&out, code).context("write dart")?;
                println!("dart → {}", out.display());
            }
            other => bail!("unknown target language: {other}"),
        }
    }

    Ok(())
}
