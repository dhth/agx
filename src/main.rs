mod app;
mod env;
mod telemetry;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await?;

    Ok(())
}
