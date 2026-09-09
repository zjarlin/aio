use std::path::Path;

use anyhow::Result;

pub struct TemplateFile {
    pub path: &'static str,
    pub content: &'static str,
    pub executable: bool,
}

pub fn materialize(
    root: &Path,
    files: &[TemplateFile],
    replacements: &[(&str, String)],
) -> Result<()> {
    for template in files {
        let relative_path = render(template.path, replacements);
        let path = root.join(relative_path);
        if let Some(parent) = path.parent() {
            super::create_directory(parent)?;
        }
        super::write(&path, &render(template.content, replacements))?;
        set_executable(&path, template.executable)?;
    }
    Ok(())
}

fn render(template: &str, replacements: &[(&str, String)]) -> String {
    replacements
        .iter()
        .fold(template.to_owned(), |rendered, (from, to)| {
            rendered.replace(from, to)
        })
}

#[cfg(unix)]
fn set_executable(path: &Path, executable: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    if executable {
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path, _executable: bool) -> Result<()> {
    Ok(())
}
