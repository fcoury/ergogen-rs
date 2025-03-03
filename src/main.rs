use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

#[derive(Parser, Debug)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Starts the web interface
    Web {
        #[clap(short, long)]
        listen_addr: Option<String>,

        #[clap(short, long)]
        port: Option<u16>,
    },

    /// Generates the output files
    #[clap(alias = "gen")]
    Generate {
        path: String,

        #[clap(short, long)]
        output: Option<String>,

        #[clap(short, long)]
        debug: bool,

        #[clap(short, long)]
        clean: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Default to INFO level if RUST_LOG is not set
            "ergogen=info,tower_http=info".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    let cli = Cli::parse();

    match cli.command {
        Command::Web { listen_addr, port } => {
            ergogen_app::start_webserver(listen_addr, port).await?;
        }

        Command::Generate { path, debug, .. } => {
            let contents = tokio::fs::read_to_string(&path).await?;
            ergogen_app::process(&contents, debug).await?;
        }
    }

    Ok(())
}
