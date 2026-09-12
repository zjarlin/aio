use anyhow::{Result, ensure};

use crate::DeliveryManifest;

pub fn parse(source: &str) -> Result<DeliveryManifest> {
    ensure!(source.len() <= 16384, "交付清单过大");
    let manifest: DeliveryManifest = toml::from_str(source)?;
    ensure!(manifest.version == 1, "不支持的交付协议版本");
    ensure!(!manifest.build.command.is_empty(), "缺少构建命令");
    ensure!(manifest.build.command.len() <= 32, "构建参数过多");
    ensure!(
        manifest
            .build
            .command
            .iter()
            .all(|arg| !arg.contains('\0') && arg.len() <= 4096),
        "构建参数无效"
    );
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_protocol_and_empty_commands() {
        for source in [
            "version=2\n[build]\nenvironment='rust'\ncommand=['sh','build.sh']",
            "version=1\n[build]\nenvironment='kotlin'\ncommand=[]",
            "version=1\n[build]\nenvironment='typescript'\ncommand=['x']\nprivileged=true",
        ] {
            assert!(parse(source).is_err());
        }
    }
}
