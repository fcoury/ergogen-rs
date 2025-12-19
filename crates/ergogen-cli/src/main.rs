use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ergogen_dxf2png::{save_dxf_as_png, RenderOptions};

#[derive(Parser)]
#[command(name = "ergogen")]
#[command(about = "Ergogen keyboard generator (Rust implementation)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a DXF file to PNG
    Dxf2png {
        /// Input DXF file path
        input: PathBuf,

        /// Output PNG file path (defaults to input with .png extension)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Width of the output image in pixels
        #[arg(short = 'W', long, default_value = "800")]
        width: u32,

        /// Height of the output image in pixels
        #[arg(short = 'H', long, default_value = "600")]
        height: u32,

        /// Padding around the drawing in pixels
        #[arg(short, long, default_value = "20")]
        padding: u32,

        /// Stroke width for lines and curves
        #[arg(short, long, default_value = "2.0")]
        stroke_width: f32,

        /// Background color as hex (e.g., ffffff for white, 000000 for black)
        #[arg(long, default_value = "ffffff")]
        bg: String,

        /// Stroke color as hex (e.g., 000000 for black, ff0000 for red)
        #[arg(long, default_value = "000000")]
        stroke: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dxf2png {
            input,
            output,
            width,
            height,
            padding,
            stroke_width,
            bg,
            stroke,
        } => {
            let output = output.unwrap_or_else(|| input.with_extension("png"));

            let background = parse_hex_color(&bg).unwrap_or_else(|e| {
                eprintln!("Invalid background color '{}': {}", bg, e);
                std::process::exit(1);
            });

            let stroke_color = parse_hex_color(&stroke).unwrap_or_else(|e| {
                eprintln!("Invalid stroke color '{}': {}", stroke, e);
                std::process::exit(1);
            });

            let opts = RenderOptions {
                width,
                height,
                padding,
                stroke_width,
                background,
                stroke_color,
                ..Default::default()
            };

            match save_dxf_as_png(&input, &output, &opts) {
                Ok(()) => {
                    println!("Converted {} -> {}", input.display(), output.display());
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn parse_hex_color(hex: &str) -> Result<[u8; 4], String> {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 && hex.len() != 8 {
        return Err("expected 6 or 8 hex digits".to_string());
    }

    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "invalid red component")?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "invalid green component")?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "invalid blue component")?;
    let a = if hex.len() == 8 {
        u8::from_str_radix(&hex[6..8], 16).map_err(|_| "invalid alpha component")?
    } else {
        255
    };

    Ok([r, g, b, a])
}
