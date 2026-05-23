use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssistantError {
    #[error("OPENAI_API_KEY 或 OPENAI_ADMIN_KEY 环境变量未配置")]
    MissingApiKey,
    #[error("页面 URL 无效：{0}")]
    InvalidUrl(String),
    #[error("页面上下文为空")]
    EmptyContext,
    #[error("页面抓取失败：{0}")]
    Fetch(String),
    #[error("页面解析失败：{0}")]
    Parse(String),
    #[error("问题不能为空")]
    EmptyQuestion,
    #[error("OpenAI 请求失败：{0}")]
    OpenAI(String),
}

pub type AssistantResult<T> = Result<T, AssistantError>;
