use std::path::Path;

use anyhow::{Context as _, Result};

const DEFAULT_WEB_PORT: u16 = 8080;
const REPOSITORY_ENV_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../.env");

/// 应用启动配置。
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub database_url: Option<String>,
}

impl AppConfig {
    /// 从仓库配置文件加载一次应用配置。
    pub fn load() -> Result<Self> {
        load_repository_env()?;

        let port = optional_env("AZ_AIO_WEB_PORT")?
            .map(|value| {
                value
                    .parse::<u16>()
                    .context("AZ_AIO_WEB_PORT 必须是有效的 u16 端口")
            })
            .transpose()?
            .unwrap_or(DEFAULT_WEB_PORT);
        let database_url = optional_env("AZ_AIO_DATABASE_URL")?;

        Ok(Self { port, database_url })
    }
}

fn load_repository_env() -> Result<()> {
    let path = Path::new(REPOSITORY_ENV_PATH);
    dotenvy::from_path_override(path)
        .with_context(|| format!("读取仓库配置文件失败: {}", path.display()))
}

fn optional_env(key: &str) -> Result<Option<String>> {
    match std::env::var(key) {
        Ok(value) => {
            let value = value.trim();
            Ok((!value.is_empty()).then(|| value.to_owned()))
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error).with_context(|| format!("环境变量 {key} 不是有效的 Unicode")),
    }
}
