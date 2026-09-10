use anyhow::Result;
use tempfile::{TempDir, tempdir};

use super::*;

#[test]
fn packages_standalone_directory_without_git_and_round_trips_file() -> Result<()> {
    let directory = plugin_directory()?;
    let output = directory.path().join("build/hello.aio-plugin");
    package(PackageOptions {
        root: directory.path().to_path_buf(),
        git: Some("https://example.com/team/hello".to_owned()),
        version: "1.0.0".to_owned(),
        output: Some(output.clone()),
    })?;
    let package = read_package(&output, None, None)?;
    assert_eq!(package.git, "https://example.com/team/hello.git");
    assert_eq!(package.source_revision, None);
    assert_eq!(package.version, "1.0.0");
    assert!(read_package(&output, Some("https://example.com/other.git"), None).is_err());
    assert!(read_package(&output, None, Some("2.0.0")).is_err());
    Ok(())
}

#[test]
fn accepts_dirty_manifest_and_ignored_uncommitted_artifact() -> Result<()> {
    let directory = plugin_directory()?;
    let root = directory.path();
    fs::write(root.join(".gitignore"), "dist/\n")?;
    git(root, &["init", "--quiet"])?;
    git(root, &["config", "user.email", "test@example.com"])?;
    git(root, &["config", "user.name", "AIO Test"])?;
    git(root, &["add", "aio-plugin.toml", ".gitignore"])?;
    git(root, &["commit", "--quiet", "-m", "source"])?;
    git(
        root,
        &[
            "remote",
            "add",
            "origin",
            "https://example.com/team/hello.git",
        ],
    )?;
    let previous = prepare_package(root, None, Some("1.0.0".to_owned()))?;
    let manifest = fs::read_to_string(root.join("aio-plugin.toml"))?;
    fs::write(
        root.join("aio-plugin.toml"),
        manifest.replace("Demo", "Uncommitted"),
    )?;
    let pages = fs::read_to_string(root.join("dist/pages.json"))?;
    fs::write(
        root.join("dist/pages.json"),
        pages.replace("Original", "New build"),
    )?;
    let updated = prepare_package(root, None, Some("1.0.0".to_owned()))?;
    assert_eq!(updated.source_revision, previous.source_revision);
    assert_eq!(updated.source_revision.as_ref().map(String::len), Some(40));
    assert_ne!(updated.rev, previous.rev);
    assert!(updated.manifest_toml.contains("Uncommitted"));
    assert!(String::from_utf8(updated.verify()?.artifact)?.contains("New build"));
    Ok(())
}

#[test]
fn requires_explicit_source_without_origin_and_infers_node_version() -> Result<()> {
    let directory = plugin_directory()?;
    fs::write(
        directory.path().join("package.json"),
        "{\"version\":\"3.2.1\"}",
    )?;
    assert!(prepare_package(directory.path(), None, None).is_err());
    let package = prepare_package(
        directory.path(),
        Some("https://example.com/p.git".to_owned()),
        None,
    )?;
    assert_eq!(package.version, "3.2.1");
    Ok(())
}

#[test]
fn does_not_inherit_parent_git_metadata() -> Result<()> {
    let parent = tempdir()?;
    git(parent.path(), &["init", "--quiet"])?;
    git(
        parent.path(),
        &["remote", "add", "origin", "https://example.com/parent.git"],
    )?;
    let plugin = parent.path().join("child");
    fs::create_dir(&plugin)?;
    write_plugin(&plugin)?;
    assert!(!own_git_repository(&plugin));
    let package = prepare_package(
        &plugin,
        Some("https://example.com/child.git".to_owned()),
        Some("1.0.0".to_owned()),
    )?;
    assert_eq!(package.source_revision, None);
    assert_eq!(package.git, "https://example.com/child.git");
    Ok(())
}

#[test]
fn packages_frontend_build_and_backend_together_without_git_or_scripts() -> Result<()> {
    let directory = plugin_directory()?;
    let root = directory.path();
    let manifest = fs::read_to_string(root.join("aio-plugin.toml"))?;
    fs::write(
        root.join("aio-plugin.toml"),
        format!("{manifest}\n[plugin.frontend]\npath='dist/web'\n"),
    )?;
    fs::create_dir_all(root.join("dist/web/assets"))?;
    fs::write(root.join("dist/web/index.html"), "<!doctype html>")?;
    fs::write(root.join("dist/web/assets/frontend.wasm"), b"\0asm")?;
    fs::write(
        root.join("dist/pages.json"),
        r#"[{"id":"hello","label":"Hello","icon":null,"scene":{"id":"workspace","label":"Workspace"},"body":{"kind":"frontend","entry":"index.html"}}]"#,
    )?;
    let package = prepare_package(
        root,
        Some("https://example.com/fullstack.git".to_owned()),
        Some("1.0.0".to_owned()),
    )?;
    assert_eq!(package.verify()?.frontend.len(), 2);
    assert_eq!(package.source_revision, None);
    let output = root.join("complete.aio-plugin");
    fs::write(&output, package.encode()?)?;
    fs::remove_dir_all(root.join("dist"))?;
    assert_eq!(read_package(&output, None, None)?, package);
    Ok(())
}

fn plugin_directory() -> Result<TempDir> {
    let directory = tempdir()?;
    write_plugin(directory.path())?;
    Ok(directory)
}

fn write_plugin(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join("dist"))?;
    fs::write(
        root.join("aio-plugin.toml"),
        "[plugin.runtime]\nkind='page-definition'\nartifact='dist/pages.json'\n[plugin.marketplace]\ntitle='Demo'\nsummary='Demo pages'\nlicense='MIT'\ntags=['test']\n[[plugin.subplugins]]\nid='hello'\npages=['hello']\n",
    )?;
    fs::write(
        root.join("dist/pages.json"),
        r#"[{"id":"hello","label":"Hello","icon":null,"scene":{"id":"workspace","label":"Workspace"},"body":{"kind":"text","title":"Hello","content":"Original"}}]"#,
    )?;
    Ok(())
}

fn git(root: &Path, arguments: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()?;
    ensure!(
        output.status.success(),
        "测试 Git 失败: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
