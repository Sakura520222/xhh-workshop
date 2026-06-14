//! axum app 构建：CORS + Trace + 路由 + Swagger UI

use std::net::SocketAddr;

use axum::middleware;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::auth_mw::require_bearer;
use crate::routes::api_routes;
use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "小黑盒本地 API",
        version = "0.1.1",
        description = "xhh-http 提供：扫码登录、发帖、评论、点赞、收藏、搜索、Agent 调用。
完整端点见源码 crates/xhh-http/src/routes/，以下为关键端点示例。"
    ),
    paths(
        crate::routes::auth::get_qrcode,
        crate::routes::auth::login,
        crate::routes::auth::status,
        crate::routes::auth::logout,
        crate::routes::agent::chat,
        crate::routes::agent::auto_post,
    ),
    components(schemas(
        crate::state::ConfigSnapshot,
        crate::routes::auth::QrQuery,
        crate::routes::auth::LoginReq,
        crate::routes::auth::LoginStatus,
        crate::routes::agent::ChatReq,
        crate::routes::agent::AutoPostReq,
    ))
)]
pub struct ApiDoc;

/// 构建完整的 axum app
pub fn build_app(state: AppState) -> axum::Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 仅当启用 Bearer Token 时挂载中间件
    let api = if state.bearer_token.is_some() {
        api_routes(state.clone()).layer(middleware::from_fn_with_state(
            state.clone(),
            require_bearer,
        ))
    } else {
        api_routes(state.clone())
    };

    let swagger = SwaggerUi::new("/docs").url("/docs/openapi.json", ApiDoc::openapi());

    axum::Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .merge(swagger)
        .merge(api)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

/// 解析监听地址（"127.0.0.1:9876" / ":9876" / "9876"）
pub fn parse_addr(addr: &str) -> Result<SocketAddr, String> {
    let addr = addr.trim();
    if addr.is_empty() {
        return Err("地址为空".into());
    }
    if let Ok(port) = addr.parse::<u16>() {
        return Ok(([127, 0, 0, 1], port).into());
    }
    let addr = if addr.starts_with(':') {
        format!("127.0.0.1{}", addr)
    } else {
        addr.to_string()
    };
    addr.parse()
        .map_err(|e: std::net::AddrParseError| e.to_string())
}
