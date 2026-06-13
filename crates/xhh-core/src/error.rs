//! 错误类型定义

use thiserror::Error;

/// 库统一错误类型
#[derive(Debug, Error)]
pub enum Error {
    #[error("网络请求失败: {0}")]
    Network(#[from] reqwest::Error),

    #[error("URL 解析失败: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("JSON 序列化/反序列化失败: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("登录凭据已失效，请重新扫码登录")]
    AuthExpired,

    #[error("未登录（找不到 pkey 或 heybox_id）")]
    NotLoggedIn,

    #[error("扫码超时（{timeout}秒）")]
    QrTimeout { timeout: u64 },

    #[error("登录响应中未找到 {0} 字段")]
    MissingCredential(&'static str),

    #[error("API 返回错误: status={status}, msg={msg}")]
    ApiError { status: String, msg: String },

    #[error("无效输入: {0}")]
    InvalidInput(String),

    #[error("上传失败: {0}")]
    UploadFailed(String),

    #[error("其他错误: {0}")]
    Other(String),
}

/// 库统一 Result 类型
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = Error::NotLoggedIn;
        assert_eq!(e.to_string(), "未登录（找不到 pkey 或 heybox_id）");
    }
}
