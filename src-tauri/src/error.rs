use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("认证已失效，请重新登录")]
    Unauthorized,
    #[error("权限不足")]
    Forbidden,
    #[error("数据不存在")]
    NotFound,
    #[error("请求参数无效：{0}")]
    BadRequest(String),
    #[error("数据冲突：{0}")]
    Conflict(String),
    #[error("数据库错误")]
    Database {
        #[from]
        source: sqlx::Error,
    },
    #[error("数据库迁移失败")]
    Migration {
        #[from]
        source: sqlx::migrate::MigrateError,
    },
    #[error("OpenAI 助手错误：{0}")]
    Assistant(String),
    #[error("路径错误")]
    Path {
        #[from]
        source: tauri::Error,
    },
    #[error("IO 错误")]
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("JSON 处理失败")]
    Json {
        #[from]
        source: serde_json::Error,
    },
    #[error("YAML 处理失败")]
    Yaml {
        #[from]
        source: serde_yml::Error,
    },
    #[error("密码处理失败")]
    Password(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: u16,
    pub message: String,
    pub details: Option<String>,
}

impl AppError {
    fn code(&self) -> u16 {
        match self {
            Self::BadRequest(_) => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict(_) => 409,
            Self::Database { .. }
            | Self::Migration { .. }
            | Self::Assistant(_)
            | Self::Path { .. }
            | Self::Io { .. }
            | Self::Json { .. }
            | Self::Yaml { .. }
            | Self::Password(_) => 500,
        }
    }

    pub fn into_command_error(self) -> CommandError {
        let details = match &self {
            Self::Database { source } => Some(source.to_string()),
            Self::Migration { source } => Some(source.to_string()),
            Self::Assistant(detail) => Some(detail.clone()),
            Self::Path { source } => Some(source.to_string()),
            Self::Io { source } => Some(source.to_string()),
            Self::Json { source } => Some(source.to_string()),
            Self::Yaml { source } => Some(source.to_string()),
            Self::Password(detail) => Some(detail.clone()),
            _ => None,
        };

        CommandError {
            code: self.code(),
            message: self.to_string(),
            details,
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
