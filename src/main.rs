use clap::Parser;
use honeytrap::cli::{Cli, Commands};
use honeytrap::http::handlers::AppState;
use honeytrap::template::TemplateStore;
use honeytrap::{http, render, template};
use std::process::ExitCode;
use std::sync::Arc;

fn init_tracing() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();
}

fn main() -> ExitCode {
    init_tracing();
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            listen,
            templates,
            secret,
            max_bytes,
            trusted_proxy,
        } => {
            let store = match TemplateStore::load(&templates) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("failed to load templates from {}: {e}", templates.display());
                    return ExitCode::FAILURE;
                }
            };
            if let Err(e) = template::validate::run(&store, &secret, max_bytes) {
                eprintln!("{e}");
                eprintln!("Reduce template size or raise --max-bytes. Refusing to start.");
                return ExitCode::FAILURE;
            }

            let state = Arc::new(AppState {
                store,
                salt: secret,
                max_bytes,
                trusted_proxy,
            });

            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("failed to start tokio runtime: {e}");
                    return ExitCode::FAILURE;
                }
            };
            if let Err(e) = rt.block_on(http::run_server(listen, state)) {
                eprintln!("server error: {e}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Commands::Gen {
            path,
            ip,
            host,
            templates,
            secret,
            max_bytes,
        } => {
            let store = match TemplateStore::load(&templates) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("failed to load templates from {}: {e}", templates.display());
                    return ExitCode::FAILURE;
                }
            };
            if let Err(e) = template::validate::run(&store, &secret, max_bytes) {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }

            match render::render_for_path(&store, &path, &ip, &host.to_lowercase(), &secret) {
                Some(Ok(rendered)) => {
                    use std::io::Write;
                    std::io::stdout().write_all(&rendered.bytes).unwrap();
                    ExitCode::SUCCESS
                }
                Some(Err(e)) => {
                    eprintln!("render error: {e}");
                    ExitCode::FAILURE
                }
                None => {
                    eprintln!("no template registered for path {path:?}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
