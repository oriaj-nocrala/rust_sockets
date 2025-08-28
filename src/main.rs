#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    archsockrust::cli::run_cli().await
}