use anyhow::Result;
use mem_mcp::MemServer;
use rmcp::transport::io::stdio;
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Logs go to stderr so they don't interfere with stdio JSON-RPC traffic.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_env("MEM_MCP_LOG").unwrap_or_else(|_| EnvFilter::new("info")))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let transport = stdio();
    let service = MemServer::new().serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
