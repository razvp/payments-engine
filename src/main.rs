use std::sync::Arc;

use anyhow::{anyhow, Context};

use payments_engine::domain::Ledger;
use payments_engine::run_csv_stream::run;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();
    let file_name = args.nth(1).ok_or(anyhow!("Input file not provided"))?;
    let input = tokio::fs::File::open(&file_name)
        .await
        .context(format!("Can't open input file: `{}`", file_name))?;
    setup_tracing();

    let ledger = Arc::new(Ledger::new());
    run(input, ledger.clone()).await;

    let mut output = std::io::stdout().lock();
    ledger.dump_to_writer(&mut output)?;
    Ok(())
}

fn setup_tracing() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "error")
    }
    tracing_subscriber::fmt::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}
