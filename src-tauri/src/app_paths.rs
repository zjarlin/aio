use std::{fs, path::Path};

use crate::capability_broker::CapabilityBroker;
use crate::error::AppResult;

pub fn open_data_dir(data_dir: &Path, broker: &CapabilityBroker) -> AppResult<String> {
    fs::create_dir_all(data_dir)?;
    broker.open_directory(data_dir)
}
