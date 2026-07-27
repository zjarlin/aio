//! 生成源码的临时编译门禁与原子替换。

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, anyhow, bail};
use az_micro_dict::contribution::DictSourceBundle;
use nature_compiler::{ArtifactFile, ArtifactSet};
use tempfile::TempDir;

const GENERATED_ENUMS_START: &str = "// nature-compiler 动态枚举开始";
const GENERATED_ENUMS_END: &str = "// nature-compiler 动态枚举结束";

/// 固定目标生成器，不接受请求侧目录参数。
#[derive(Clone, Debug)]
pub struct ArtifactGate {
    output_root: PathBuf,
}

impl ArtifactGate {
    pub fn new(output_root: impl Into<PathBuf>) -> Self {
        Self {
            output_root: output_root.into(),
        }
    }

    pub fn output_root(&self) -> &Path {
        &self.output_root
    }

    /// 在隔离 crate 通过全部门禁后替换当前生成目录。
    pub fn verify_and_publish(
        &self,
        artifacts: &ArtifactSet,
        dictionary_bundle: Option<&DictSourceBundle>,
    ) -> anyhow::Result<ArtifactSet> {
        let (temp, formatted) = self.prepare(artifacts, dictionary_bundle)?;
        run_cargo(temp.path(), &["fmt", "--all", "--", "--check"])?;
        run_cargo(temp.path(), &["check", "--all-targets"])?;
        run_cargo(temp.path(), &["test", "--all-targets"])?;
        run_cargo(temp.path(), &["clippy", "--all-targets"])?;
        self.publish(&formatted, dictionary_bundle)?;
        Ok(formatted)
    }

    /// 只执行确定性 rustfmt，供 CI 逐字节对比提交生成物。
    pub fn format_artifacts(
        &self,
        artifacts: &ArtifactSet,
        dictionary_bundle: Option<&DictSourceBundle>,
    ) -> anyhow::Result<ArtifactSet> {
        self.prepare(artifacts, dictionary_bundle)
            .map(|(_, formatted)| formatted)
    }

    fn prepare(
        &self,
        artifacts: &ArtifactSet,
        dictionary_bundle: Option<&DictSourceBundle>,
    ) -> anyhow::Result<(TempDir, ArtifactSet)> {
        let temp = TempDir::new().context("创建 nature 临时编译目录失败")?;
        write_text(
            &temp.path().join("Cargo.toml"),
            r#"[package]
name = "az-aio-nature-generated"
version = "0.0.0"
edition = "2024"

[dependencies]
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
az-dict-macros = { git = "https://github.com/zjarlin/addzero-lib-rust.git", rev = "0209ab56aa57a4aa6102ce497cb5d82b6266e5ec" }
az-dict-spec = { git = "https://github.com/zjarlin/addzero-lib-rust.git", rev = "0209ab56aa57a4aa6102ce497cb5d82b6266e5ec" }
derive_more = { version = "2", features = ["display"] }
"#,
        )?;
        for file in &artifacts.files {
            write_artifact(temp.path(), file, &artifacts.hash)?;
        }
        if let Some(bundle) = dictionary_bundle {
            write_dictionary_bundle(temp.path(), bundle)?;
        }
        run_cargo(temp.path(), &["fmt", "--all"])?;
        let formatted = collect_formatted_artifacts(temp.path(), artifacts)?;
        Ok((temp, formatted))
    }

    fn publish(
        &self,
        artifacts: &ArtifactSet,
        dictionary_bundle: Option<&DictSourceBundle>,
    ) -> anyhow::Result<()> {
        let parent = self
            .output_root
            .parent()
            .ok_or_else(|| anyhow!("生成目录没有父目录: {}", self.output_root.display()))?;
        let hash_prefix = artifacts
            .hash
            .get(..12)
            .ok_or_else(|| anyhow!("artifact hash 长度不足"))?;
        let stage = parent.join(format!(".nature-stage-{hash_prefix}"));
        let backup = parent.join(format!(".nature-backup-{hash_prefix}"));
        remove_directory_if_present(&stage)?;
        remove_directory_if_present(&backup)?;
        fs::create_dir_all(&stage)
            .with_context(|| format!("创建生成暂存目录失败: {}", stage.display()))?;

        copy_file(
            &self.output_root.join("Cargo.toml"),
            &stage.join("Cargo.toml"),
        )?;
        copy_file(
            &self.output_root.join("blueprint-source.txt"),
            &stage.join("blueprint-source.txt"),
        )?;
        copy_directory(&self.output_root.join("specs"), &stage.join("specs"))?;
        for file in &artifacts.files {
            if file.relative_path == "src/enums.rs" {
                continue;
            }
            write_artifact(&stage, file, &artifacts.hash)?;
        }
        let repository_enums = fs::read_to_string(self.output_root.join("src/enums.rs"))
            .context("读取当前公共枚举源码失败")?;
        let blueprint_enums = artifacts
            .files
            .iter()
            .find(|file| file.relative_path == "src/enums.rs")
            .map(|file| file.source.as_str())
            .unwrap_or("");
        let dictionary_enums = dictionary_bundle
            .and_then(|bundle| bundle_file(bundle, "enums.rs"))
            .unwrap_or("");
        let merged_enums =
            merge_repository_enums(&repository_enums, blueprint_enums, dictionary_enums);
        write_text(&stage.join("src/enums.rs"), &merged_enums)?;
        if let Some(bundle) = dictionary_bundle {
            write_dictionary_specs(&stage, bundle)?;
        }

        fs::rename(&self.output_root, &backup).with_context(|| {
            format!(
                "备份当前生成目录失败: {} -> {}",
                self.output_root.display(),
                backup.display()
            )
        })?;
        if let Err(error) = fs::rename(&stage, &self.output_root) {
            fs::rename(&backup, &self.output_root).context("恢复旧生成目录失败")?;
            return Err(error).context("替换生成目录失败");
        }
        remove_directory_if_present(&backup)?;
        Ok(())
    }
}

fn collect_formatted_artifacts(
    root: &Path,
    artifacts: &ArtifactSet,
) -> anyhow::Result<ArtifactSet> {
    let mut files = Vec::new();
    for file in &artifacts.files {
        let path = root.join(&file.relative_path);
        let mut source = fs::read_to_string(&path)
            .with_context(|| format!("读取格式化生成物失败: {}", path.display()))?;
        if file.relative_path == "src/lib.rs"
            && let Some((generated, _)) = source.split_once("pub const ARTIFACT_HASH")
        {
            source = format!("{}\n", generated.trim_end());
        }
        files.push(ArtifactFile {
            relative_path: file.relative_path.clone(),
            source,
        });
    }
    Ok(ArtifactSet::new(files))
}

fn write_dictionary_bundle(root: &Path, bundle: &DictSourceBundle) -> anyhow::Result<()> {
    let enum_source = bundle_file(bundle, "enums.rs").unwrap_or("");
    let enum_path = root.join("src/enums.rs");
    let current = fs::read_to_string(&enum_path)
        .with_context(|| format!("读取临时枚举文件失败: {}", enum_path.display()))?;
    write_text(&enum_path, &format!("{current}\n{enum_source}"))?;
    write_dictionary_specs(root, bundle)
}

fn write_dictionary_specs(root: &Path, bundle: &DictSourceBundle) -> anyhow::Result<()> {
    for file in &bundle.files {
        if !file.relative_path.starts_with("specs") {
            continue;
        }
        write_text(&root.join("src").join(&file.relative_path), &file.source)?;
    }
    Ok(())
}

fn bundle_file<'a>(bundle: &'a DictSourceBundle, path: &str) -> Option<&'a str> {
    bundle
        .files
        .iter()
        .find(|file| file.relative_path == Path::new(path))
        .map(|file| file.source.as_str())
}

fn merge_repository_enums(
    repository_source: &str,
    blueprint_source: &str,
    dictionary_source: &str,
) -> String {
    let foundation = repository_source
        .split_once(GENERATED_ENUMS_START)
        .map(|(foundation, _)| foundation.trim_end())
        .unwrap_or_else(|| repository_source.trim_end());
    let generated = [blueprint_source.trim(), dictionary_source.trim()]
        .into_iter()
        .filter(|source| !source.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if generated.is_empty() {
        format!("{foundation}\n\n{GENERATED_ENUMS_START}\n{GENERATED_ENUMS_END}\n")
    } else {
        format!("{foundation}\n\n{GENERATED_ENUMS_START}\n{generated}\n{GENERATED_ENUMS_END}\n")
    }
}

fn write_artifact(root: &Path, file: &ArtifactFile, artifact_hash: &str) -> anyhow::Result<()> {
    let destination = root.join(&file.relative_path);
    let source = if file.relative_path == "src/lib.rs" {
        format!(
            "{}\npub const ARTIFACT_HASH: &str = \"{}\";\n",
            file.source, artifact_hash
        )
    } else {
        file.source.clone()
    };
    write_text(&destination, &source)
}

fn write_text(path: &Path, source: &str) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("生成文件没有父目录: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("创建生成文件目录失败: {}", parent.display()))?;
    fs::write(path, source).with_context(|| format!("写入生成文件失败: {}", path.display()))
}

fn run_cargo(root: &Path, arguments: &[&str]) -> anyhow::Result<()> {
    let output = Command::new("cargo")
        .args(arguments)
        .current_dir(root)
        .output()
        .with_context(|| format!("执行 cargo {} 失败", arguments.join(" ")))?;
    if output.status.success() {
        return Ok(());
    }
    bail!(
        "cargo {} 门禁失败:\n{}\n{}",
        arguments.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn copy_directory(source: &Path, destination: &Path) -> anyhow::Result<()> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(destination)
        .with_context(|| format!("创建复制目录失败: {}", destination.display()))?;
    for entry in
        fs::read_dir(source).with_context(|| format!("读取目录失败: {}", source.display()))?
    {
        let entry = entry.context("读取生成目录项失败")?;
        let file_type = entry.file_type().context("读取生成目录项类型失败")?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if file_type.is_file() {
            copy_file(&entry.path(), &target)?;
        }
    }
    Ok(())
}

fn copy_file(source: &Path, destination: &Path) -> anyhow::Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("复制目标没有父目录: {}", destination.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("创建复制目标目录失败: {}", parent.display()))?;
    fs::copy(source, destination)
        .with_context(|| format!("复制生成文件失败: {}", source.display()))?;
    Ok(())
}

fn remove_directory_if_present(path: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_dir_all(path).with_context(|| format!("清理目录失败: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_gate_does_not_replace_current_output() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let output = temp.path().join("nature");
        fs::create_dir_all(output.join("src"))?;
        write_text(
            &output.join("Cargo.toml"),
            "[package]\nname='kept'\nversion='0.1.0'\n",
        )?;
        write_text(&output.join("src/enums.rs"), "pub enum Kept {}\n")?;
        write_text(&output.join("src/lib.rs"), "pub const KEPT: bool = true;\n")?;
        let artifacts = ArtifactSet::new(vec![ArtifactFile {
            relative_path: "src/lib.rs".to_string(),
            source: "这不是 Rust".to_string(),
        }]);

        let result = ArtifactGate::new(&output).verify_and_publish(&artifacts, None);

        assert!(result.is_err());
        assert!(fs::read_to_string(output.join("src/lib.rs"))?.contains("KEPT"));
        Ok(())
    }

    #[test]
    fn empty_generated_enums_do_not_create_unstable_whitespace() {
        let repository = "pub enum Kept {}\n\n// nature-compiler 动态枚举开始\n\n\n// nature-compiler 动态枚举结束\n";

        let merged = merge_repository_enums(repository, "\n", "");

        assert_eq!(
            merged,
            "pub enum Kept {}\n\n// nature-compiler 动态枚举开始\n// nature-compiler 动态枚举结束\n"
        );
    }
}
