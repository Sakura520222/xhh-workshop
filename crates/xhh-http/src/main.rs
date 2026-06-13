//! xhh-serve: HTTP 服务入口
//!
//! 启动一个本地 REST 服务（默认 `127.0.0.1:9876`），对外暴露小黑盒全部 API。
//! Swagger UI 在 `/docs`。

use std::net::SocketAddr;

use anyhow::{anyhow, Result};
use xhh_http::{build_app, parse_addr, AppState};

#[derive(Debug, clap::Parser)]
#[command(name = "xhh-serve", version, about = "小黑盒本地 HTTP REST 服务")]
struct Args {
    /// 监听地址，如 `127.0.0.1:9876` / `:9876` / `9876`
    #[arg(short = 'b', long, default_value = "127.0.0.1:9876")]
    bind: String,

    /// 启用 Bearer Token 鉴权（默认读 XHH_BEARER_TOKEN 环境变量）
    #[arg(short = 't', long)]
    token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "xhh_http=info,xhh_core=warn,info".into()),
        )
        .init();

    let args: Args = clap::Parser::parse();
    let addr: SocketAddr = parse_addr(&args.bind).map_err(|e| anyhow!("地址解析失败: {}", e))?;
    let token = args
        .token
        .or_else(|| std::env::var("XHH_BEARER_TOKEN").ok())
        .filter(|s| !s.is_empty());
    let state = AppState::new(token.clone());

    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bearer_hint = if token.is_some() {
        "（已启用 Bearer Token 鉴权）"
    } else {
        "（未启用鉴权，本地可访问）"
    };
    tracing::info!("xhh-serve 监听 http://{} {}", addr, bearer_hint);
    tracing::info!("Swagger UI: http://{}/docs", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
