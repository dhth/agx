mod app;
mod domain;
mod env;
mod helpers;
mod session;
mod telemetry;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await?;

    Ok(())
}
