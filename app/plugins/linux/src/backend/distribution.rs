use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinuxDistribution {
    Ubuntu,
}

impl LinuxDistribution {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Ubuntu => "ubuntu",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Ubuntu => "Ubuntu",
        }
    }
}
