use std::{fs, path::Path, process::Command};

use crate::error::AppResult;

pub fn open_data_dir(data_dir: &Path) -> AppResult<String> {
    fs::create_dir_all(data_dir)?;
    open_directory(data_dir)?;
    Ok(data_dir.to_string_lossy().into_owned())
}

fn open_directory(path: &Path) -> AppResult<()> {
    let mut command = platform_open_command(path)?;
    command.spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn platform_open_command(path: &Path) -> AppResult<Command> {
    let mut command = Command::new("open");
    command.arg(path);
    Ok(command)
}

#[cfg(target_os = "windows")]
fn platform_open_command(path: &Path) -> AppResult<Command> {
    let mut command = Command::new("explorer");
    command.arg(path);
    Ok(command)
}

#[cfg(target_os = "linux")]
fn platform_open_command(path: &Path) -> AppResult<Command> {
    let mut command = Command::new("xdg-open");
    command.arg(path);
    Ok(command)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn platform_open_command(_path: &Path) -> AppResult<Command> {
    Err(crate::error::AppError::BadRequest(
        "当前系统不支持打开数据目录".to_string(),
    ))
}
