use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 渲染器可直接消费的组件节点。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UiNode {
    /// Rudi provider 的编译时模块路径，同时也是组件 canonical ID。
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// 跨渲染器的增量界面操作流。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum UiOp {
    Open {
        node: UiNode,
    },
    Leaf {
        node: UiNode,
    },
    Text {
        value: String,
    },
    Close {
        kind: String,
    },
    Patch {
        id: String,
        attributes: BTreeMap<String, String>,
    },
}
