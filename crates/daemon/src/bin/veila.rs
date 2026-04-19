#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = veila_daemon::DaemonOptions::parse_control_args(std::env::args())?;
    veila_daemon::run_control(options).await
}
