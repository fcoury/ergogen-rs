use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use ergogen_dxf2png::{RenderOptions, save_dxf_as_png};

mod error;
mod render;

use error::{CliError, ErrorCode};

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
    /// Render a config (YAML) into outlines/pcbs/cases outputs
    Render(RenderArgs),
}

#[derive(Args)]
struct RenderArgs {
    /// Input config path (file) or bundle folder (containing config.yaml)
    input: PathBuf,

    /// Output folder (default: ./output)
    #[arg(short, long, default_value = "output")]
    output: PathBuf,

    /// Additionally write modern JSCAD v2 outputs (compatible with jscad.app)
    #[arg(long)]
    jscad_v2: bool,

    /// Include debug outputs (source/, points/, and `_` prefixed definitions)
    #[arg(short = 'd', long)]
    debug: bool,

    /// Delete the output folder before rendering
    #[arg(long)]
    clean: bool,

    /// Generate SVG outputs for outlines
    #[arg(long)]
    svg: bool,
}

fn main() -> ExitCode {
    run()
}

fn run() -> ExitCode {
    let cli = match Cli::try_parse_from(normalize_args(std::env::args_os().collect())) {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            let code = match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => 0,
                _ => ErrorCode::Usage as u8,
            };
            return ExitCode::from(code);
        }
    };

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

            let background = match parse_hex_color(&bg) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "{}",
                        CliError::usage(format!("Invalid background color '{bg}': {e}"))
                    );
                    return ExitCode::from(ErrorCode::Usage as u8);
                }
            };

            let stroke_color = match parse_hex_color(&stroke) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "{}",
                        CliError::usage(format!("Invalid stroke color '{stroke}': {e}"))
                    );
                    return ExitCode::from(ErrorCode::Usage as u8);
                }
            };

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
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}", CliError::input(format!("Error: {e}")));
                    ExitCode::from(ErrorCode::Input as u8)
                }
            }
        }
        Commands::Render(RenderArgs {
            input,
            output,
            jscad_v2,
            debug,
            clean,
            svg,
        }) => match render::run_render(input, output, debug, clean, jscad_v2, svg) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("Error: {err}");
                ExitCode::from(err.code as u8)
            }
        },
    }
}

fn normalize_args(mut args: Vec<OsString>) -> Vec<OsString> {
    if should_insert_render(&args) {
        args.insert(1, OsString::from("render"));
    }
    args
}

fn should_insert_render(args: &[OsString]) -> bool {
    let mut saw_double_dash = false;
    for arg in args.iter().skip(1) {
        let s = arg.to_string_lossy();
        if !saw_double_dash && s == "--" {
            saw_double_dash = true;
            continue;
        }
        if !saw_double_dash && s.starts_with('-') {
            continue;
        }
        if s == "render" || s == "dxf2png" {
            return false;
        }
        return true;
    }
    false
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
