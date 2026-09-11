use super::*;

fn manifest() -> Result<RepositoryManifest> {
    crate::parse_manifest(
        "[plugin.runtime]\nkind='page-definition'\nartifact='dist/pages.json'\n[plugin.frontend]\npath='dist/web'\n[[plugin.subplugins]]\nid='screen'\npages=['screen']\n",
    )
}

#[test]
fn rejects_nonportable_and_escaping_frontend_paths() {
    for path in [
        "",
        "../secret",
        "/index.html",
        "a//b",
        "a/./b",
        "a/../b",
        "a\\b",
        "C:/secret",
        ".env",
        "a/.git/config",
        "a?b",
        "a#b",
        "a%2fb",
        "vendors/@scope/../../secret",
        "vendors/@scope/%2e%2e/secret",
        "https://user@host/file.js",
        "a\0b",
        "assets/NUL.txt",
        "assets/COM1.js",
        "assets/file.",
        "__aio_bridge.js",
    ] {
        assert!(validate_frontend_path(path).is_err(), "{path:?}");
    }
    assert!(validate_frontend_path("assets/screen-42_bg.wasm").is_ok());
    assert!(validate_frontend_path("vendors/@js-joda/core/dist/js-joda.js").is_ok());
}

#[test]
fn collects_scoped_package_assets() -> Result<()> {
    let root = tempfile::tempdir()?;
    let asset = "vendors/@js-joda/core/dist/js-joda.js";
    fs::create_dir_all(root.path().join("dist/web/vendors/@js-joda/core/dist"))?;
    fs::write(root.path().join("dist/web").join(asset), "export {}")?;
    assert!(frontend_files(root.path(), &manifest()?)?.contains_key(asset));
    Ok(())
}

#[test]
fn validates_real_frontend_entries_and_declared_contributions() -> Result<()> {
    let root = tempfile::tempdir()?;
    fs::create_dir_all(root.path().join("dist/web/assets"))?;
    fs::write(root.path().join("dist/web/index.html"), "<!doctype html>")?;
    fs::write(root.path().join("dist/web/assets/app.wasm"), b"\0asm")?;
    let files = frontend_files(root.path(), &manifest()?)?;
    assert_eq!(
        files.keys().map(String::as_str).collect::<Vec<_>>(),
        ["assets/app.wasm", "index.html"]
    );
    let mut pages = serde_json::from_str::<Vec<PageDefinition>>(
        r#"[{"id":"screen","label":"Screen","icon":null,"scene":{"id":"community","label":"Community"},"body":{"kind":"frontend","entry":"index.html"}}]"#,
    )?;
    crate::validate_page_definitions(&pages)?;
    crate::validate_declared_pages(&manifest()?, &pages)?;
    validate_frontend_pages(&manifest()?, &pages, files.keys().map(String::as_str))?;
    pages[0].body = PageBody::Frontend {
        entry: "missing.html".to_owned(),
    };
    assert!(
        validate_frontend_pages(&manifest()?, &pages, files.keys().map(String::as_str)).is_err()
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn rejects_symlinks_at_every_frontend_boundary() -> Result<()> {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir()?;
    let external = tempfile::tempdir()?;
    fs::create_dir_all(root.path().join("dist/web"))?;
    fs::write(external.path().join("secret"), "secret")?;
    symlink(
        external.path().join("secret"),
        root.path().join("dist/web/index.html"),
    )?;
    assert!(frontend_files(root.path(), &manifest()?).is_err());
    fs::remove_file(root.path().join("dist/web/index.html"))?;
    fs::remove_dir(root.path().join("dist/web"))?;
    symlink(external.path(), root.path().join("dist/web"))?;
    assert!(frontend_files(root.path(), &manifest()?).is_err());
    Ok(())
}

#[test]
fn rejects_undeclared_or_source_frontends() -> Result<()> {
    assert!(
        crate::parse_manifest("[plugin.client]\npath='client'\n[plugin.frontend]\npath='dist/web'")
            .is_err()
    );
    let pages = serde_json::from_str::<Vec<PageDefinition>>(
        r#"[{"id":"screen","label":"Screen","icon":null,"scene":{"id":"community","label":"Community"},"body":{"kind":"frontend","entry":"index.html"}}]"#,
    )?;
    let mut manifest = manifest()?;
    manifest.plugin.frontend = None;
    assert!(crate::validate_declared_pages(&manifest, &pages).is_err());
    Ok(())
}
