use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

#[test]
fn public_unit_business_enums_only_exist_in_generated_crate() -> Result<()> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut violations = Vec::new();
    scan_rust_files(&workspace.join("platform"), &mut violations)?;
    scan_rust_files(&workspace.join("plugins"), &mut violations)?;
    scan_rust_files(&workspace.join("crates"), &mut violations)?;
    if !violations.is_empty() {
        bail!(
            "公开无数据业务枚举必须进入 crates/generated/nature/src/enums.rs:\n{}",
            violations.join("\n")
        );
    }
    Ok(())
}

fn scan_rust_files(path: &Path, violations: &mut Vec<String>) -> Result<()> {
    if path.ends_with("crates/generated/nature") || path.ends_with("target") {
        return Ok(());
    }
    for entry in fs::read_dir(path).with_context(|| format!("读取目录失败: {}", path.display()))?
    {
        let entry = entry.context("读取源码目录项失败")?;
        let path = entry.path();
        if path.is_dir() {
            scan_rust_files(&path, violations)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .with_context(|| format!("读取 Rust 源码失败: {}", path.display()))?;
        for line in source
            .lines()
            .filter(|line| line.trim_start().starts_with("pub enum "))
        {
            let allowed = [
                "HookCommand",
                "UiOp",
                "PropertyValue",
                "EdgeAuthError",
                "PageRenderTarget",
                "GeneratedOperationPlanStep",
                "OperationExecutorDefinition",
                "OperationPlanStep",
            ]
            .iter()
            .any(|name| line.contains(name));
            if !allowed {
                violations.push(format!("{}: {}", path.display(), line.trim()));
            }
        }
    }
    Ok(())
}
