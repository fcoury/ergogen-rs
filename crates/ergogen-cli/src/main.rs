use std::path::PathBuf;
use anyhow::{Result, Context};
use clap::Parser;
use serde_json::Value;
use ergogen_parser::Units;
use ergogen_layout::{PointsConfig, generate};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Input configuration file (YAML or JSON)
    #[arg(value_name = "CONFIG")]
    input: PathBuf,

    /// Output directory
    #[arg(short, long, value_name = "DIR", default_value = "output")]
    output: PathBuf,

    /// Enable debug mode
    #[arg(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let content = std::fs::read_to_string(&cli.input)
        .with_context(|| format!("Failed to read input file: {:?}", cli.input))?;

    let config: Value = serde_yml::from_str(&content)
        .with_context(|| "Failed to parse YAML configuration")?;

    let units = Units::default();
    
    // Extract points config
    if let Some(points_val) = config.get("points") {
        let points_cfg: PointsConfig = serde_json::from_value(points_val.clone())
            .with_context(|| "Failed to parse points configuration")?;
        
        let points = generate(&points_cfg, &units)
            .with_context(|| "Failed to generate points")?;
        
        println!("Generated {} points.", points.len());
        if cli.debug {
            for (name, p) in points {
                println!("{}: ({:.2}, {:.2}, {:.2})", name, p.x, p.y, p.r);
            }
        }
    } else {
        println!("No 'points' section found in configuration.");
    }

    Ok(())
}