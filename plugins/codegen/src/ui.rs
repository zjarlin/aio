#![allow(non_snake_case)]

//! nature-compiler 母语工作台页面。

use az_aio_platform::plugin::contract::NativeRenderContext;
use dioxus::prelude::*;
use registry::ui::{
    button::Button,
    card::{Card, CardContent, CardHeader, CardTitle},
};

use crate::{
    contract::UI_ACTION_PATH,
    source_template::{select_source_template, source_templates},
};

pub fn NatureCompilerPage(context: NativeRenderContext) -> Element {
    let revision = query_value(&context.active_route, "revision");
    let error = query_value(&context.active_route, "error");
    let template_key = query_value(&context.active_route, "template");
    let selected_template = select_source_template(template_key.as_deref());
    let output_root = generated_output_root();
    rsx! {
        div { class: "space-y-5",
            if let Some(revision_id) = revision {
                div { class: "flex flex-wrap items-center justify-between gap-3 border-l-4 border-green-600 bg-green-600/10 px-4 py-3 text-sm",
                    span { class: "min-w-0 break-all",
                        "Revision 已进入生成队列："
                        code { "{revision_id}" }
                    }
                    a {
                        class: "shrink-0 font-medium underline underline-offset-4",
                        href: revision_href(&revision_id),
                        target: "_blank",
                        rel: "noreferrer",
                        "查看状态"
                    }
                }
                section {
                    id: "nature-observability",
                    class: "grid gap-4 border border-border rounded p-4",
                    "data-revision-id": "{revision_id}",
                    div { class: "flex flex-wrap items-center justify-between gap-3",
                        h2 { class: "text-base font-semibold", "编译观测" }
                        button {
                            id: "nature-observability-refresh",
                            class: "inline-flex h-9 w-9 items-center justify-center rounded border border-border hover:bg-muted",
                            r#type: "button",
                            title: "刷新编译状态",
                            aria_label: "刷新编译状态",
                            "↻"
                        }
                    }
                    div {
                        id: "nature-observability-summary",
                        class: "flex flex-wrap gap-2 text-sm",
                        "加载中"
                    }
                    div {
                        id: "nature-observability-metrics",
                        class: "grid gap-3 md:grid-cols-3"
                    }
                    ol {
                        id: "nature-observability-timeline",
                        class: "grid gap-2 list-none p-0 m-0"
                    }
                    pre {
                        id: "nature-observability-error",
                        hidden: true,
                        class: "overflow-x-auto whitespace-pre-wrap border-l-4 border-destructive bg-destructive/10 p-3 text-sm text-destructive"
                    }
                    section {
                        id: "nature-generated-files",
                        hidden: true,
                        class: "grid min-w-0 gap-3 border-t border-border pt-4",
                        "data-output-root": "{output_root}",
                        div { class: "flex flex-wrap items-center justify-between gap-3",
                            div { class: "grid min-w-0 gap-1",
                                h3 { class: "text-sm font-semibold", "生成文件" }
                                code {
                                    id: "nature-generated-path",
                                    class: "break-all text-xs text-muted-foreground",
                                    "{output_root}"
                                }
                            }
                            button {
                                id: "nature-copy-generated-path",
                                class: "inline-flex h-9 w-9 shrink-0 items-center justify-center rounded border border-border hover:bg-muted",
                                r#type: "button",
                                title: "复制生成目录",
                                aria_label: "复制生成目录",
                                "⧉"
                            }
                        }
                        div { class: "grid min-w-0 gap-3 lg:grid-cols-[15rem_minmax(0,1fr)]",
                            nav {
                                id: "nature-generated-file-list",
                                class: "grid max-h-[32rem] content-start gap-1 overflow-auto border-r border-border pr-3",
                                aria_label: "生成文件"
                            }
                            pre {
                                id: "nature-generated-file-preview",
                                class: "max-h-[32rem] min-h-64 min-w-0 overflow-auto whitespace-pre p-4 text-xs bg-muted/40",
                                "选择文件后在这里预览"
                            }
                        }
                    }
                }
                script { dangerous_inner_html: OBSERVABILITY_SCRIPT }
            }
            if let Some(message) = error {
                div { class: "border-l-4 border-destructive bg-destructive/10 px-4 py-3 text-sm text-destructive",
                    "{message}"
                }
            }
            Card {
                CardHeader { CardTitle { "nature-compiler" } }
                CardContent {
                    form { method: "post", action: UI_ACTION_PATH, class: "grid gap-4",
                        div { class: "grid gap-2 text-sm",
                            span { class: "font-medium", "业务模板" }
                            nav {
                                class: "inline-flex w-fit max-w-full overflow-x-auto rounded border border-border p-1",
                                aria_label: "业务模板",
                                for template in source_templates() {
                                    a {
                                        class: if template.key == selected_template.key {
                                            "min-w-24 px-3 py-2 text-center font-medium bg-primary text-primary-foreground rounded-sm"
                                        } else {
                                            "min-w-24 px-3 py-2 text-center text-muted-foreground hover:bg-muted rounded-sm"
                                        },
                                        href: template_href(template.key),
                                        aria_current: if template.key == selected_template.key { "page" } else { "false" },
                                        "{template.label}"
                                    }
                                }
                            }
                        }
                        label { class: "grid gap-2 text-sm",
                            span { class: "font-medium", "项目" }
                            input {
                                class: "aio-input",
                                name: "project_id",
                                value: selected_template.project_id,
                                required: true,
                            }
                        }
                        label { class: "grid gap-2 text-sm",
                            span { class: "font-medium", "母语需求与建模" }
                            textarea {
                                class: "aio-input font-mono",
                                name: "source_text",
                                rows: "18",
                                required: true,
                                "{selected_template.source_text}"
                            }
                        }
                        div { class: "flex justify-end",
                            Button { button_type: "submit", "生成 Revision" }
                        }
                    }
                }
            }
        }
    }
}

const OBSERVABILITY_SCRIPT: &str = r#"
(() => {
  const root = document.getElementById('nature-observability');
  if (!root) return;

  const revisionId = root.dataset.revisionId;
  const summary = document.getElementById('nature-observability-summary');
  const metrics = document.getElementById('nature-observability-metrics');
  const timeline = document.getElementById('nature-observability-timeline');
  const errorBox = document.getElementById('nature-observability-error');
  const refresh = document.getElementById('nature-observability-refresh');
  const filesRoot = document.getElementById('nature-generated-files');
  const fileList = document.getElementById('nature-generated-file-list');
  const filePreview = document.getElementById('nature-generated-file-preview');
  const copyPath = document.getElementById('nature-copy-generated-path');
  const activeStatuses = new Set(['queued', 'running', 'checking']);
  const statusLabels = {
    queued: '排队中',
    running: '运行中',
    checking: '门禁检查',
    succeeded: '生成成功',
    failed: '生成失败',
    published: '已发布'
  };
  let timer;

  function clear(element) {
    while (element.firstChild) element.removeChild(element.firstChild);
  }

  function textElement(tag, text, className) {
    const element = document.createElement(tag);
    element.textContent = text;
    if (className) element.className = className;
    return element;
  }

  function duration(value) {
    if (value === null || value === undefined) return '进行中';
    if (value < 1000) return `${value} ms`;
    return `${(value / 1000).toFixed(2)} s`;
  }

  function metric(label, value) {
    const item = document.createElement('div');
    item.className = 'min-w-0 grid gap-1 border-l-2 border-border pl-3';
    item.appendChild(textElement('span', label, 'text-xs text-muted-foreground'));
    item.style.minWidth = '0';
    const metricValue = textElement('strong', value || '-', 'text-sm');
    metricValue.style.minWidth = '0';
    metricValue.style.overflowWrap = 'anywhere';
    item.appendChild(metricValue);
    metrics.appendChild(item);
  }

  function render(data) {
    clear(summary);
    clear(metrics);
    clear(timeline);

    const status = statusLabels[data.status] || data.status;
    summary.appendChild(textElement('strong', status));
    summary.appendChild(textElement('code', data.id, 'break-all text-muted-foreground'));

    const runs = data.runs || [];
    const run = runs.length > 0 ? runs[runs.length - 1] : null;
    const events = run ? run.events || [] : [];
    const compileEvent = events.find((event) => event.stage === 'compile');
    const inference = compileEvent && compileEvent.metadata
      ? compileEvent.metadata.inference
      : null;

    metric('模型', inference && inference.model ? inference.model : inference && inference.engine);
    metric('输入 Token', inference ? String(inference.inputTokens) : '-');
    metric('输出 Token', inference ? String(inference.outputTokens) : '-');
    metric('缓存 Token', inference ? String(inference.cachedInputTokens) : '-');
    metric('运行耗时', run ? duration(run.durationMs) : '-');
    metric('Artifact', data.artifactHash || '-');

    if (events.length === 0) {
      timeline.appendChild(textElement('li', '该运行没有阶段事件', 'text-sm text-muted-foreground'));
    }
    for (const event of events) {
      const item = document.createElement('li');
      item.className = event.parentEventId
        ? 'ml-6 grid gap-1 border-l-2 border-border py-2 pl-3'
        : 'grid gap-1 border-l-4 border-border py-2 pl-3';
      const header = document.createElement('div');
      header.className = 'flex flex-wrap items-center justify-between gap-2 text-sm';
      header.appendChild(textElement('strong', event.label));
      header.appendChild(textElement('span', `${statusLabels[event.status] || event.status} · ${duration(event.durationMs)}`, 'text-muted-foreground'));
      item.appendChild(header);
      if (event.message) item.appendChild(textElement('pre', event.message, 'whitespace-pre-wrap text-sm text-destructive'));
      timeline.appendChild(item);
    }

    const error = data.errorMessage || (run && run.errorMessage);
    errorBox.textContent = error || '';
    errorBox.hidden = !error;

    renderFiles(data.generatedFiles || []);

    if (timer) window.clearTimeout(timer);
    if (activeStatuses.has(data.status)) timer = window.setTimeout(load, 1000);
  }

  function renderFiles(files) {
    clear(fileList);
    filesRoot.hidden = files.length === 0;
    if (files.length === 0) {
      filePreview.textContent = '';
      return;
    }
    let selected;
    const selectFile = (file, button) => {
      if (selected) selected.removeAttribute('data-selected');
      selected = button;
      selected.setAttribute('data-selected', 'true');
      for (const item of fileList.querySelectorAll('button')) {
        item.className = item === selected
          ? 'w-full border-l-2 border-primary bg-muted px-3 py-2 text-left text-xs font-medium'
          : 'w-full border-l-2 border-transparent px-3 py-2 text-left text-xs text-muted-foreground hover:bg-muted';
      }
      filePreview.textContent = file.source;
    };
    for (const file of files) {
      const button = document.createElement('button');
      button.type = 'button';
      button.textContent = file.path;
      button.title = `预览 ${file.path}`;
      button.addEventListener('click', () => selectFile(file, button));
      fileList.appendChild(button);
    }
    const preferred = files.find((file) => file.path === 'application.json') || files[0];
    const preferredButton = [...fileList.querySelectorAll('button')]
      .find((button) => button.textContent === preferred.path);
    if (preferredButton) selectFile(preferred, preferredButton);
  }

  async function load() {
    refresh.disabled = true;
    try {
      const response = await fetch(`/api/nature/revisions/${encodeURIComponent(revisionId)}`);
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const payload = await response.json();
      if (!payload.data) throw new Error(payload.msg || 'revision 数据为空');
      render(payload.data);
    } catch (error) {
      errorBox.textContent = String(error);
      errorBox.hidden = false;
    } finally {
      refresh.disabled = false;
    }
  }

  refresh.addEventListener('click', load);
  copyPath.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(filesRoot.dataset.outputRoot);
      copyPath.title = '已复制生成目录';
    } catch (error) {
      errorBox.textContent = `复制目录失败：${String(error)}`;
      errorBox.hidden = false;
    }
  });
  load();
})();
"#;

fn generated_output_root() -> String {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/generated/nature");
    match path.canonicalize() {
        Ok(path) => path.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

fn template_href(template_key: &str) -> String {
    let route = format!("/nature?template={}", urlencoding::encode(template_key));
    format!("/?route={}", urlencoding::encode(&route))
}

fn revision_href(revision_id: &str) -> String {
    format!("/api/nature/revisions/{}", urlencoding::encode(revision_id))
}

fn query_value(route: &str, key: &str) -> Option<String> {
    let query = route.split_once('?')?.1;
    for pair in query.split('&') {
        let (pair_key, pair_value) = pair.split_once('=').unwrap_or((pair, ""));
        if pair_key == key {
            return Some(
                urlencoding::decode(pair_value)
                    .map(|value| value.into_owned())
                    .unwrap_or_else(|_| pair_value.to_string()),
            );
        }
    }
    None
}
