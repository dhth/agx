mod app;
mod domain;
mod env;
mod helpers;
mod providers;
mod session;
mod telemetry;
mod tools;
mod debug;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await?;

    Ok(())
}
