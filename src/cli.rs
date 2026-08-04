use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "honeytrap",
    about = "Serves fabricated credentials to credential-harvesting scanners"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the HTTP server.
    Serve {
        #[arg(long, default_value = "127.0.0.1:8090")]
        listen: SocketAddr,
        #[arg(long, default_value = "./templates")]
        templates: PathBuf,
        /// Salt mixed into the per-(ip,host) RNG seed. Required: must stay
        /// stable across restarts for determinism to hold.
        #[arg(long)]
        secret: String,
        #[arg(long, default_value_t = 8192)]
        max_bytes: usize,
        /// Trust X-Forwarded-For for client IP (set when behind Caddy).
        #[arg(long, default_value_t = false)]
        trusted_proxy: bool,
    },
    /// Render a single response to stdout, reproducing what `serve` would
    /// have sent for the same (path, ip, host, secret).
    Gen {
        #[arg(long)]
        path: String,
        #[arg(long)]
        ip: String,
        #[arg(long)]
        host: String,
        #[arg(long, default_value = "./templates")]
        templates: PathBuf,
        #[arg(long)]
        secret: String,
        #[arg(long, default_value_t = 8192)]
        max_bytes: usize,
    },
}
