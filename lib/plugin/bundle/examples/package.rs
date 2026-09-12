use std::path::Path;

use anyhow::{Result, ensure};
use az_plugin_bundle::Bundle;

fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    ensure!(
        arguments.len() == 6,
        "用法: package <仓库> <相对清单路径> <HTTPS Git> <完整 SHA> <SemVer> <输出文件>"
    );
    let bundle = Bundle::from_directory(
        Path::new(&arguments[0]),
        &arguments[1],
        arguments[2].clone(),
        arguments[3].clone(),
        arguments[4].clone(),
    )?;
    let bytes = bundle.encode()?;
    Bundle::decode(&bytes)?;
    std::fs::write(&arguments[5], &bytes)?;
    println!(
        "v2 bundle={} bytes={} digest={}",
        arguments[5],
        bytes.len(),
        bundle.digest
    );
    Ok(())
}
