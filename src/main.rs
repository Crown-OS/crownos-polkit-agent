#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    crownos_polkit_agent::agent::daemon::run().await
}
