use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};

const DEFAULT_WEB_PORT: u16 = 8080;
const REPOSITORY_ENV_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../.env");
const DATABASE_URL_ENV: &str = "AZ_AIO_DATABASE_URL";
const DATABASE_URL_OVERRIDE_ENV: &str = "AZ_AIO_DATABASE_URL_OVERRIDE";
const DATABASE_MIGRATIONS_ENABLED_ENV: &str = "AZ_AIO_DATABASE_MIGRATIONS_ENABLED";
const WEB_DIST_ENV: &str = "AZ_AIO_WEB_DIST";

/// 应用启动配置。
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub database_url: Option<String>,
    pub database_migrations_enabled: bool,
    pub web_dist_dir: Option<PathBuf>,
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
        let database_url = resolve_database_url(
            optional_env(DATABASE_URL_OVERRIDE_ENV)?,
            optional_env(DATABASE_URL_ENV)?,
        );
        let database_migrations_enabled = optional_env(DATABASE_MIGRATIONS_ENABLED_ENV)?
            .map(|value| parse_bool(DATABASE_MIGRATIONS_ENABLED_ENV, &value))
            .transpose()?
            .unwrap_or(true);
        let web_dist_dir = optional_env(WEB_DIST_ENV)?.map(PathBuf::from);

        Ok(Self {
            port,
            database_url,
            database_migrations_enabled,
            web_dist_dir,
        })
    }

    #[cfg(test)]
    pub fn for_test() -> Self {
        Self {
            port: 0,
            database_url: None,
            database_migrations_enabled: true,
            web_dist_dir: None,
        }
    }
}

fn resolve_database_url(
    override_url: Option<String>,
    configured_url: Option<String>,
) -> Option<String> {
    override_url.or(configured_url)
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

fn parse_bool(key: &str, value: &str) -> Result<bool> {
    value
        .parse::<bool>()
        .with_context(|| format!("环境变量 {key} 必须是 true 或 false"))
}

#[cfg(test)]
mod tests {
    use super::{parse_bool, resolve_database_url};

    #[test]
    fn database_url_override_has_priority() {
        let configured_url = Some("postgresql://configured/aio".to_owned());
        let override_url = Some("postgresql://tunnel/aio".to_owned());

        assert_eq!(
            resolve_database_url(override_url, configured_url),
            Some("postgresql://tunnel/aio".to_owned())
        );
    }

    #[test]
    fn migration_switch_accepts_strict_boolean_values() -> anyhow::Result<()> {
        assert!(parse_bool("TEST", "true")?);
        assert!(!parse_bool("TEST", "false")?);
        assert!(parse_bool("TEST", "yes").is_err());
        Ok(())
    }
}
