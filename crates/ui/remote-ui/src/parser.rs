use std::{collections::BTreeMap, mem, sync::Arc};

use anyhow::{Context, Result, bail, ensure};

use crate::{ComponentIndex, ComponentShape, UiNode, UiOp};

const DEFAULT_MAX_BUFFER: usize = 1024 * 1024;
const DEFAULT_MAX_DEPTH: usize = 64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ParseState {
    #[default]
    Text,
    Tag,
}

/// 把任意分片边界的紧凑 DSL 增量解析为 `UiOp`。
#[derive(Debug)]
pub struct UiParser {
    components: Arc<ComponentIndex>,
    state: ParseState,
    tag_buffer: String,
    text_buffer: String,
    stack: Vec<String>,
    quoted: bool,
    escaped: bool,
    max_buffer: usize,
    max_depth: usize,
}

impl UiParser {
    /// 使用 Rudi 派生的组件索引创建解析器。
    #[must_use]
    pub fn new(components: Arc<ComponentIndex>) -> Self {
        Self {
            components,
            state: ParseState::Text,
            tag_buffer: String::new(),
            text_buffer: String::new(),
            stack: Vec::new(),
            quoted: false,
            escaped: false,
            max_buffer: DEFAULT_MAX_BUFFER,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }

    /// 创建带资源限制的流式解析器。
    #[must_use]
    pub fn with_limits(
        components: Arc<ComponentIndex>,
        max_buffer: usize,
        max_depth: usize,
    ) -> Self {
        Self {
            max_buffer,
            max_depth,
            ..Self::new(components)
        }
    }

    /// 输入一个 DSL 分片并立即返回本次完成的操作。
    pub fn feed(&mut self, chunk: &str) -> Result<Vec<UiOp>> {
        let mut operations = Vec::new();
        for character in chunk.chars() {
            match self.state {
                ParseState::Text => self.consume_text(character, &mut operations)?,
                ParseState::Tag => self.consume_tag(character, &mut operations)?,
            }
        }
        if self.state == ParseState::Text {
            self.flush_text(&mut operations);
        }
        Ok(operations)
    }

    /// 完成当前流并校验所有标签均已闭合。
    pub fn finish(&mut self) -> Result<Vec<UiOp>> {
        ensure!(
            self.state == ParseState::Text,
            "远程 UI DSL 在标签中途结束: [{}",
            self.tag_buffer
        );
        let mut operations = Vec::new();
        self.flush_text(&mut operations);
        if let Some(canonical_id) = self.stack.last() {
            bail!("远程 UI DSL 存在未闭合组件: {canonical_id}");
        }
        Ok(operations)
    }

    fn consume_text(&mut self, character: char, operations: &mut Vec<UiOp>) -> Result<()> {
        if character == '[' {
            self.flush_text(operations);
            self.state = ParseState::Tag;
            return Ok(());
        }
        self.text_buffer.push(character);
        self.ensure_buffer_limit()
    }

    fn consume_tag(&mut self, character: char, operations: &mut Vec<UiOp>) -> Result<()> {
        if self.escaped {
            self.tag_buffer.push(character);
            self.escaped = false;
            return self.ensure_buffer_limit();
        }
        if character == '\\' && self.quoted {
            self.tag_buffer.push(character);
            self.escaped = true;
            return self.ensure_buffer_limit();
        }
        if character == '"' {
            self.quoted = !self.quoted;
            self.tag_buffer.push(character);
            return self.ensure_buffer_limit();
        }
        if character == ']' && !self.quoted {
            self.complete_tag(operations)?;
            self.state = ParseState::Text;
            return Ok(());
        }
        self.tag_buffer.push(character);
        self.ensure_buffer_limit()
    }

    fn complete_tag(&mut self, operations: &mut Vec<UiOp>) -> Result<()> {
        let raw = mem::take(&mut self.tag_buffer);
        self.quoted = false;
        self.escaped = false;
        let raw = raw.trim();
        ensure!(!raw.is_empty(), "远程 UI DSL 不允许空标签");

        if let Some(close_name) = raw.strip_prefix('/') {
            return self.close_tag(close_name.trim(), operations);
        }
        let tokens = tokenize(raw).context("解析远程 UI 标签失败")?;
        ensure!(!tokens.is_empty(), "远程 UI DSL 不允许空标签");
        if tokens[0] == "upd" {
            return self.patch_tag(&tokens[1..], operations);
        }
        self.open_or_leaf_tag(&tokens, operations)
    }

    fn close_tag(&mut self, dsl_name: &str, operations: &mut Vec<UiOp>) -> Result<()> {
        let component = self.components.resolve(dsl_name)?;
        let canonical_id = component.canonical_id().to_string();
        let Some(open_id) = self.stack.pop() else {
            bail!("远程 UI DSL 出现多余闭合标签: [/{dsl_name}]");
        };
        ensure!(
            canonical_id == open_id,
            "远程 UI DSL 标签闭合顺序错误: 期望关闭 {open_id}，实际关闭 {canonical_id}"
        );
        operations.push(UiOp::Close { kind: canonical_id });
        Ok(())
    }

    fn patch_tag(&self, tokens: &[String], operations: &mut Vec<UiOp>) -> Result<()> {
        let (mut attributes, content) = parse_attributes(tokens);
        ensure!(content.is_empty(), "[upd] 不接受正文内容");
        let id = attributes
            .remove("id")
            .filter(|value| !value.is_empty())
            .context("[upd] 必须提供非空 id")?;
        operations.push(UiOp::Patch { id, attributes });
        Ok(())
    }

    fn open_or_leaf_tag(&mut self, tokens: &[String], operations: &mut Vec<UiOp>) -> Result<()> {
        let component = self.components.resolve(&tokens[0])?;
        let canonical_id = component.canonical_id().to_string();
        let dsl_name = component.dsl_name().to_string();
        let shape = component.shape();
        let (mut attributes, content) = parse_attributes(&tokens[1..]);
        let id = attributes.remove("id").filter(|value| !value.is_empty());
        let content = (!content.is_empty()).then(|| content.join(" "));
        let opens_container = match shape {
            ComponentShape::Leaf => false,
            ComponentShape::Container => {
                ensure!(
                    content.is_none(),
                    "容器组件必须以无正文开标签开始: {dsl_name}"
                );
                true
            }
            ComponentShape::Dual => content.is_none(),
        };
        let node = UiNode {
            kind: canonical_id.clone(),
            id,
            attributes,
            content,
        };
        if !opens_container {
            operations.push(UiOp::Leaf { node });
            return Ok(());
        }

        ensure!(
            self.stack.len() < self.max_depth,
            "远程 UI DSL 嵌套深度超过限制: {}",
            self.max_depth
        );
        self.stack.push(canonical_id);
        operations.push(UiOp::Open { node });
        Ok(())
    }

    fn flush_text(&mut self, operations: &mut Vec<UiOp>) {
        let value = mem::take(&mut self.text_buffer);
        if !value.trim().is_empty() {
            operations.push(UiOp::Text { value });
        }
    }

    fn ensure_buffer_limit(&self) -> Result<()> {
        let buffered = self.tag_buffer.len() + self.text_buffer.len();
        ensure!(
            buffered <= self.max_buffer,
            "远程 UI DSL 缓冲区超过限制: {} 字节",
            self.max_buffer
        );
        Ok(())
    }
}

fn parse_attributes(tokens: &[String]) -> (BTreeMap<String, String>, Vec<String>) {
    let mut attributes = BTreeMap::new();
    let mut content = Vec::new();
    for token in tokens {
        if let Some((key, value)) = token.split_once(':')
            && valid_attribute_name(key)
        {
            attributes.insert(key.to_string(), value.to_string());
            continue;
        }
        if matches!(
            token.as_str(),
            "disabled" | "required" | "checked" | "multiple" | "readonly"
        ) {
            attributes.insert(token.to_string(), "true".to_string());
            continue;
        }
        content.push(token.clone());
    }
    (attributes, content)
}

fn valid_attribute_name(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && characters
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn tokenize(raw: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quoted = false;
    let mut escaped = false;

    for character in raw.chars() {
        if escaped {
            token.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' && quoted {
            escaped = true;
            continue;
        }
        if character == '"' {
            quoted = !quoted;
            continue;
        }
        if character.is_whitespace() && !quoted {
            if !token.is_empty() {
                tokens.push(mem::take(&mut token));
            }
            continue;
        }
        token.push(character);
    }

    ensure!(!quoted, "标签属性存在未闭合双引号");
    ensure!(!escaped, "标签属性以未完成转义结尾");
    if !token.is_empty() {
        tokens.push(token);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use rudi::Context as RudiContext;

    use super::*;

    fn parser() -> Result<UiParser> {
        let mut context = RudiContext::auto_register();
        let components = ComponentIndex::from_context(&mut context)?;
        Ok(UiParser::new(Arc::new(components)))
    }

    #[test]
    fn parses_nested_components_across_arbitrary_chunks() -> Result<()> {
        let mut parser = parser()?;
        let mut operations = Vec::new();
        for chunk in ["[car", "d][card-title 设备", "状态][/card]"] {
            operations.extend(parser.feed(chunk)?);
        }
        operations.extend(parser.finish()?);

        // 任意网络分片不能改变最终组件操作顺序。
        assert_eq!(operations.len(), 3);
        let UiOp::Open { node: card } = &operations[0] else {
            bail!("第一个操作应打开 card");
        };
        assert!(card.kind.ends_with("::components::card"));
        assert!(matches!(operations[1], UiOp::Leaf { .. }));
        let UiOp::Close { kind } = &operations[2] else {
            bail!("第三个操作应关闭 card");
        };
        assert_eq!(kind, &card.kind);
        Ok(())
    }

    #[test]
    fn keeps_quoted_attribute_spaces_and_emits_patch() -> Result<()> {
        let mut parser = parser()?;
        let operations = parser.feed(
            "[input id:search label:\"设备名称\" ph:\"输入编号或名称\"][upd id:search ph:\"继续输入\"]",
        )?;
        parser.finish()?;

        let UiOp::Leaf { node } = &operations[0] else {
            bail!("第一个操作应为叶子节点");
        };
        // 带空格的展示文本必须完整保留，不能被拆成正文。
        assert_eq!(
            node.attributes.get("ph").map(String::as_str),
            Some("输入编号或名称")
        );
        let UiOp::Patch { id, attributes } = &operations[1] else {
            bail!("第二个操作应为更新操作");
        };
        assert_eq!(id, "search");
        assert_eq!(attributes.get("ph").map(String::as_str), Some("继续输入"));
        Ok(())
    }

    #[test]
    fn rejects_mismatched_closing_tag() -> Result<()> {
        let mut parser = parser()?;
        let error = match parser.feed("[card][row][/card]") {
            Ok(_) => bail!("闭合顺序错误必须失败"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("标签闭合顺序错误"));
        Ok(())
    }

    #[test]
    fn enforces_depth_limit() -> Result<()> {
        let base = parser()?;
        let mut parser = UiParser::with_limits(Arc::clone(&base.components), 1024, 1);
        let error = match parser.feed("[card][row]") {
            Ok(_) => bail!("超过嵌套限制必须失败"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("嵌套深度超过限制"));
        Ok(())
    }

    #[test]
    fn preserves_text_whitespace_across_chunks() -> Result<()> {
        let mut parser = parser()?;
        let mut operations = parser.feed("[code]hello ")?;
        operations.extend(parser.feed("world[/code]")?);
        parser.finish()?;

        // 分片边界处的空格属于正文，不能因增量 flush 被吞掉。
        assert!(matches!(
            &operations[1],
            UiOp::Text { value } if value == "hello "
        ));
        assert!(matches!(
            &operations[2],
            UiOp::Text { value } if value == "world"
        ));
        Ok(())
    }

    #[test]
    fn rejects_unregistered_component_without_from_str_fallback() -> Result<()> {
        let mut parser = parser()?;
        let error = match parser.feed("[unknown]") {
            Ok(_) => bail!("未注册组件必须失败"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("未注册的远程 UI 组件"));
        Ok(())
    }
}
