use std::{collections::BTreeSet, sync::Arc};

use az_asset_hub_contract::{AssetSummary, AssetUpsertInput, ScannedSkillSummary};
use az_ui_components::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    data_table::{DataTable, DataTableCellContext, DataTableColumn, DataTableFixed},
    dialog::{Dialog, DialogDescription, DialogTitle},
    input::Input,
};
use dioxus::prelude::*;
use icons::{Pencil, Plus, RefreshCw, Search, X};
use rudi::Singleton;
use az_dioxus_admin_shell::{
    ConventionPageContext, ConventionPageProvider, DynConventionPageProvider,
};
use studio::browser_http::{format_unix_timestamp, get_api, post_api};

#[derive(Clone, Debug, Default)]
struct Page;

impl ConventionPageProvider for Page {
    fn key(&self) -> &'static str {
        module_path!()
    }

    fn render(&self, context: ConventionPageContext) -> Element {
        rsx! {
            AssetWorkspace { title: context.page.title }
        }
    }
}

#[Singleton(name = module_path!())]
fn convention_page() -> DynConventionPageProvider {
    Arc::new(Page)
}

#[derive(Clone, Debug, PartialEq)]
enum AssetFormTarget {
    New,
    Edit(AssetSummary),
    Import(ScannedSkillSummary),
}

#[derive(Clone, Debug, PartialEq)]
struct AssetNotice {
    success: bool,
    text: String,
}

impl AssetNotice {
    fn success(text: impl Into<String>) -> Self {
        Self {
            success: true,
            text: text.into(),
        }
    }

    fn error(text: impl Into<String>) -> Self {
        Self {
            success: false,
            text: text.into(),
        }
    }
}

#[component]
fn AssetWorkspace(title: String) -> Element {
    let mut asset_generation = use_signal(|| 0_u64);
    let mut scan_generation = use_signal(|| 0_u64);
    let mut asset_search = use_signal(String::new);
    let mut asset_status = use_signal(|| "all".to_string());
    let mut asset_kind = use_signal(|| "all".to_string());
    let mut skill_search = use_signal(String::new);
    let mut dialog = use_signal(|| None::<AssetFormTarget>);
    let mut notice = use_signal(|| None::<AssetNotice>);
    let assets = use_resource(move || {
        let _generation = asset_generation();
        async move { get_api::<Vec<AssetSummary>>("", "/api/asset-hub/assets").await }
    });
    let scanned_skills = use_resource(move || {
        let _generation = scan_generation();
        async move { get_api::<Vec<ScannedSkillSummary>>("", "/api/asset-hub/skills").await }
    });
    let asset_result = assets.read().as_ref().cloned();
    let asset_rows = asset_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let scanned_result = scanned_skills.read().as_ref().cloned();
    let scanned_rows = scanned_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let registered_ids = asset_rows
        .iter()
        .map(|asset| asset.id.clone())
        .collect::<BTreeSet<_>>();
    let imported_skill_count = scanned_rows
        .iter()
        .filter(|skill| registered_ids.contains(&skill.id))
        .count();
    let asset_kinds = asset_rows
        .iter()
        .map(|asset| asset.kind.clone())
        .collect::<BTreeSet<_>>();
    let normalized_asset_search = asset_search().trim().to_lowercase();
    let selected_status = asset_status();
    let selected_kind = asset_kind();
    let visible_assets = asset_rows
        .iter()
        .filter(|asset| {
            let matches_search = normalized_asset_search.is_empty()
                || asset.title.to_lowercase().contains(&normalized_asset_search)
                || asset.id.to_lowercase().contains(&normalized_asset_search)
                || asset.source.to_lowercase().contains(&normalized_asset_search);
            let matches_status = selected_status == "all" || asset.status == selected_status;
            let matches_kind = selected_kind == "all" || asset.kind == selected_kind;
            matches_search && matches_status && matches_kind
        })
        .cloned()
        .collect::<Vec<_>>();
    let normalized_skill_search = skill_search().trim().to_lowercase();
    let visible_skills = scanned_rows
        .iter()
        .filter(|skill| {
            normalized_skill_search.is_empty()
                || skill.name.to_lowercase().contains(&normalized_skill_search)
                || skill.id.to_lowercase().contains(&normalized_skill_search)
                || skill.source.to_lowercase().contains(&normalized_skill_search)
                || skill
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&normalized_skill_search))
        })
        .cloned()
        .collect::<Vec<_>>();
    let asset_empty_text = match asset_result.as_ref() {
        None => "正在加载资产注册表".to_string(),
        Some(Err(error)) => error.clone(),
        Some(Ok(_))
            if !normalized_asset_search.is_empty()
                || selected_status != "all"
                || selected_kind != "all" =>
        {
            "没有匹配的注册资产".to_string()
        }
        Some(Ok(_)) => "暂无注册资产".to_string(),
    };
    let skill_empty_text = match scanned_result.as_ref() {
        None => "正在扫描本地技能".to_string(),
        Some(Err(error)) => error.clone(),
        Some(Ok(_)) if !normalized_skill_search.is_empty() => "没有匹配的技能".to_string(),
        Some(Ok(_)) => "未扫描到技能".to_string(),
    };

    rsx! {
        section { class: "aio-asset-workbench",
            header { class: "aio-asset-workbench__header",
                div {
                    h2 { "{title}" }
                    p { "PostgreSQL 资产注册表与本地技能发现" }
                }
                div { class: "aio-asset-workbench__header-actions",
                    Button {
                        r#type: "button",
                        size: ButtonSize::IconSm,
                        variant: ButtonVariant::Outline,
                        title: "刷新资产与技能",
                        aria_label: "刷新资产与技能",
                        onclick: move |_| {
                            notice.set(None);
                            asset_generation.with_mut(|value| {
                                *value = value.saturating_add(1);
                            });
                            scan_generation.with_mut(|value| {
                                *value = value.saturating_add(1);
                            });
                        },
                        RefreshCw { class: "size-4" }
                    }
                    Button {
                        r#type: "button",
                        onclick: move |_| {
                            notice.set(None);
                            dialog.set(Some(AssetFormTarget::New));
                        },
                        Plus { class: "size-4" }
                        "新建资产"
                    }
                }
            }
            if let Some(message) = notice() {
                div {
                    class: if message.success {
                        "aio-asset-notice"
                    } else {
                        "aio-asset-notice is-error"
                    },
                    role: if message.success { "status" } else { "alert" },
                    "{message.text}"
                }
            }
            div { class: "aio-asset-stats", aria_label: "资产中心统计",
                span { strong { "{asset_rows.len()}" } "注册资产" }
                span { strong { "{scanned_rows.len()}" } "扫描技能" }
                span { strong { "{imported_skill_count}" } "已导入技能" }
            }
            div { class: "aio-asset-workbench__grid",
                section { class: "aio-asset-panel",
                    header { class: "aio-asset-panel__header",
                        div {
                            h3 { "资产注册表" }
                            span { "{visible_assets.len()} / {asset_rows.len()}" }
                        }
                    }
                    div { class: "aio-asset-filters",
                        div { class: "aio-asset-search",
                            Search { class: "size-4" }
                            Input {
                                class: "aio-input",
                                aria_label: "搜索注册资产",
                                placeholder: "名称、标识或来源",
                                value: asset_search(),
                                oninput: move |event: FormEvent| asset_search.set(event.value()),
                            }
                        }
                        select {
                            class: "aio-input",
                            aria_label: "筛选资产类型",
                            value: asset_kind(),
                            onchange: move |event: FormEvent| asset_kind.set(event.value()),
                            option { value: "all", "全部类型" }
                            for kind in &asset_kinds {
                                option { value: kind.clone(), "{asset_kind_label(kind)}" }
                            }
                        }
                        select {
                            class: "aio-input",
                            aria_label: "筛选资产状态",
                            value: asset_status(),
                            onchange: move |event: FormEvent| asset_status.set(event.value()),
                            option { value: "all", "全部状态" }
                            option { value: "active", "有效" }
                            option { value: "inactive", "停用" }
                            option { value: "archived", "归档" }
                        }
                    }
                    DataTable::<AssetSummary> {
                        key: "asset-registry",
                        class: "aio-asset-data-table",
                        aria_label: "资产注册表".to_string(),
                        rows: visible_assets,
                        columns: asset_columns(),
                        max_height: "clamp(16rem, calc(100vh - 25rem), 38rem)",
                        empty_text: asset_empty_text,
                        row_key: |row: AssetSummary| row.id.clone(),
                        render_cell: move |cell: DataTableCellContext<AssetSummary>| {
                            asset_cell(cell, dialog, notice)
                        },
                    }
                }
                section { class: "aio-asset-panel aio-asset-panel--skills",
                    header { class: "aio-asset-panel__header",
                        div {
                            h3 { "本地技能扫描" }
                            span { "{visible_skills.len()} / {scanned_rows.len()}" }
                        }
                        Badge { variant: BadgeVariant::Outline, "~/.agents/skills" }
                    }
                    div { class: "aio-asset-filters aio-asset-filters--skills",
                        div { class: "aio-asset-search",
                            Search { class: "size-4" }
                            Input {
                                class: "aio-input",
                                aria_label: "搜索扫描技能",
                                placeholder: "名称、标签或来源",
                                value: skill_search(),
                                oninput: move |event: FormEvent| skill_search.set(event.value()),
                            }
                        }
                    }
                    DataTable::<ScannedSkillSummary> {
                        key: "scanned-skills",
                        class: "aio-asset-data-table",
                        aria_label: "本地技能扫描表".to_string(),
                        rows: visible_skills,
                        columns: scanned_skill_columns(),
                        max_height: "clamp(16rem, calc(100vh - 25rem), 38rem)",
                        empty_text: skill_empty_text,
                        row_key: |row: ScannedSkillSummary| row.id.clone(),
                        render_cell: {
                            let registered_ids = registered_ids.clone();
                            move |cell: DataTableCellContext<ScannedSkillSummary>| {
                                let imported = registered_ids.contains(&cell.row.id);
                                scanned_skill_cell(cell, imported, dialog, notice)
                            }
                        },
                    }
                }
            }
        }
        if let Some(target) = dialog() {
            AssetFormDialog {
                key: "{asset_form_key(&target)}",
                target,
                dialog,
                notice,
                asset_generation,
            }
        }
    }
}

fn asset_columns() -> Vec<DataTableColumn> {
    vec![
        DataTableColumn::leaf("title", "资产名称")
            .width(200)
            .fixed(DataTableFixed::Left),
        DataTableColumn::leaf("kind", "类型").width(96),
        DataTableColumn::leaf("source", "来源").width(220),
        DataTableColumn::leaf("updated_at", "更新时间").width(168),
        DataTableColumn::leaf("status", "状态").width(88),
        DataTableColumn::leaf("actions", "操作")
            .width(88)
            .fixed(DataTableFixed::Right),
    ]
}

fn scanned_skill_columns() -> Vec<DataTableColumn> {
    vec![
        DataTableColumn::leaf("name", "技能名称")
            .width(184)
            .fixed(DataTableFixed::Left),
        DataTableColumn::leaf("tags", "标签").width(220),
        DataTableColumn::leaf("source", "来源").width(260),
        DataTableColumn::leaf("status", "状态").width(88),
        DataTableColumn::leaf("actions", "操作")
            .width(104)
            .fixed(DataTableFixed::Right),
    ]
}

fn asset_cell(
    cell: DataTableCellContext<AssetSummary>,
    mut dialog: Signal<Option<AssetFormTarget>>,
    mut notice: Signal<Option<AssetNotice>>,
) -> Element {
    let row = cell.row;
    match cell.column.key.as_str() {
        "title" => rsx! {
            div { class: "aio-asset-identity",
                strong { "{row.title}" }
                code { "{row.id}" }
            }
        },
        "kind" => rsx! { Badge { variant: BadgeVariant::Outline, "{asset_kind_label(&row.kind)}" } },
        "source" => rsx! { code { class: "aio-asset-source", title: "{row.source}", "{row.source}" } },
        "updated_at" => rsx! { "{format_unix_timestamp(&row.updated_at)}" },
        "status" => asset_status_badge(&row.status),
        "actions" => {
            let row_for_edit = row.clone();
            rsx! {
                div { class: "aio-asset-row-actions",
                    Button {
                        r#type: "button",
                        size: ButtonSize::IconSm,
                        variant: ButtonVariant::Ghost,
                        title: "编辑资产",
                        aria_label: "编辑资产 {row.title}",
                        onclick: move |_| {
                            notice.set(None);
                            dialog.set(Some(AssetFormTarget::Edit(row_for_edit.clone())));
                        },
                        Pencil { class: "size-4" }
                    }
                }
            }
        }
        _ => rsx! { "" },
    }
}

fn scanned_skill_cell(
    cell: DataTableCellContext<ScannedSkillSummary>,
    imported: bool,
    mut dialog: Signal<Option<AssetFormTarget>>,
    mut notice: Signal<Option<AssetNotice>>,
) -> Element {
    let row = cell.row;
    match cell.column.key.as_str() {
        "name" => rsx! {
            div { class: "aio-asset-identity",
                strong { "{row.name}" }
                code { "{row.id}" }
            }
        },
        "tags" => rsx! {
            span { class: "aio-asset-tags", "{skill_tags_label(&row.tags)}" }
        },
        "source" => rsx! { code { class: "aio-asset-source", title: "{row.source}", "{row.source}" } },
        "status" => rsx! { Badge { variant: BadgeVariant::Secondary, "已扫描" } },
        "actions" => {
            let row_for_import = row.clone();
            rsx! {
                Button {
                    r#type: "button",
                    size: ButtonSize::Xs,
                    variant: if imported {
                        ButtonVariant::Outline
                    } else {
                        ButtonVariant::Primary
                    },
                    aria_label: if imported {
                        format!("更新技能资产 {}", row.name)
                    } else {
                        format!("导入技能资产 {}", row.name)
                    },
                    onclick: move |_| {
                        notice.set(None);
                        dialog.set(Some(AssetFormTarget::Import(row_for_import.clone())));
                    },
                    if imported {
                        RefreshCw { class: "size-4" }
                        "更新"
                    } else {
                        Plus { class: "size-4" }
                        "导入"
                    }
                }
            }
        }
        _ => rsx! { "" },
    }
}

fn asset_kind_label(kind: &str) -> &str {
    match kind {
        "skill" => "技能",
        "model" => "模型",
        "api" => "API",
        "document" => "文档",
        "prompt" => "提示词",
        "tool" => "工具",
        _ => kind,
    }
}

fn asset_status_badge(status: &str) -> Element {
    match status {
        "active" => rsx! { Badge { variant: BadgeVariant::Secondary, "有效" } },
        "inactive" => rsx! { Badge { variant: BadgeVariant::Outline, "停用" } },
        "archived" => rsx! { Badge { variant: BadgeVariant::Outline, "归档" } },
        _ => rsx! { Badge { variant: BadgeVariant::Outline, "{status}" } },
    }
}

fn skill_tags_label(tags: &[String]) -> String {
    if tags.is_empty() {
        "无标签".to_string()
    } else {
        tags.join(" · ")
    }
}

#[component]
fn AssetFormDialog(
    target: AssetFormTarget,
    mut dialog: Signal<Option<AssetFormTarget>>,
    mut notice: Signal<Option<AssetNotice>>,
    mut asset_generation: Signal<u64>,
) -> Element {
    let (heading, description, initial_id, id_locked, initial_kind, initial_title, initial_status, initial_source) =
        match &target {
            AssetFormTarget::New => (
                "新建资产",
                "登记一个可被低代码页面与接口引用的正式资产",
                String::new(),
                false,
                "skill".to_string(),
                String::new(),
                "active".to_string(),
                "asset-hub".to_string(),
            ),
            AssetFormTarget::Edit(asset) => (
                "编辑资产",
                "更新资产分类、来源和生命周期状态",
                asset.id.clone(),
                true,
                asset.kind.clone(),
                asset.title.clone(),
                asset.status.clone(),
                asset.source.clone(),
            ),
            AssetFormTarget::Import(skill) => (
                "导入技能资产",
                "确认扫描结果后写入 PostgreSQL 资产注册表",
                skill.id.clone(),
                true,
                skill.asset_type.clone(),
                skill.name.clone(),
                "active".to_string(),
                skill.source.clone(),
            ),
        };
    let saved_message = match target {
        AssetFormTarget::New => "资产已创建",
        AssetFormTarget::Edit(_) => "资产已更新",
        AssetFormTarget::Import(_) => "技能资产已导入",
    };
    let mut id = use_signal(move || initial_id);
    let mut kind = use_signal(move || initial_kind);
    let mut title = use_signal(move || initial_title);
    let mut status = use_signal(move || initial_status);
    let mut source = use_signal(move || initial_source);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        Dialog {
            class: "aio-asset-dialog",
            open: true,
            on_open_change: move |open: bool| {
                if !open {
                    dialog.set(None);
                }
            },
            header { class: "aio-asset-dialog__header",
                div {
                    DialogTitle { "{heading}" }
                    DialogDescription { "{description}" }
                }
                Button {
                    r#type: "button",
                    size: ButtonSize::IconSm,
                    variant: ButtonVariant::Ghost,
                    title: "关闭资产编辑",
                    aria_label: "关闭资产编辑",
                    onclick: move |_| dialog.set(None),
                    X { class: "size-4" }
                }
            }
            form {
                class: "aio-asset-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    let normalized_kind = kind().trim().to_string();
                    let normalized_title = title().trim().to_string();
                    let normalized_source = source().trim().to_string();
                    if normalized_kind.is_empty()
                        || normalized_title.is_empty()
                        || normalized_source.is_empty()
                    {
                        error.set(Some("资产名称、类型和来源不能为空".to_string()));
                        return;
                    }
                    let normalized_id = id().trim().to_string();
                    let input = AssetUpsertInput {
                        id: if normalized_id.is_empty() {
                            None
                        } else {
                            Some(normalized_id)
                        },
                        kind: normalized_kind,
                        title: normalized_title,
                        status: status(),
                        source: normalized_source,
                    };
                    saving.set(true);
                    error.set(None);
                    notice.set(None);
                    spawn(async move {
                        match post_api::<_, AssetSummary>(
                            "",
                            "/api/asset-hub/asset",
                            &input,
                        )
                        .await
                        {
                            Ok(_) => {
                                asset_generation.with_mut(|value| {
                                    *value = value.saturating_add(1);
                                });
                                notice.set(Some(AssetNotice::success(saved_message)));
                                dialog.set(None);
                            }
                            Err(message) => {
                                saving.set(false);
                                error.set(Some(message.clone()));
                                notice.set(Some(AssetNotice::error(message)));
                            }
                        }
                    });
                },
                div { class: "aio-asset-form__grid",
                    if id_locked {
                        div { class: "aio-asset-form__identity aio-asset-form__wide",
                            span { "资产标识" }
                            code { "{id}" }
                        }
                    } else {
                        label { class: "aio-asset-form__wide",
                            span { "资产标识" }
                            Input {
                                class: "aio-input",
                                aria_label: "资产标识",
                                placeholder: "可选，留空自动生成",
                                value: id(),
                                oninput: move |event: FormEvent| id.set(event.value()),
                            }
                        }
                    }
                    label {
                        span { "资产名称" }
                        Input {
                            class: "aio-input",
                            aria_label: "资产名称",
                            required: true,
                            value: title(),
                            oninput: move |event: FormEvent| title.set(event.value()),
                        }
                    }
                    label {
                        span { "资产类型" }
                        Input {
                            class: "aio-input",
                            aria_label: "资产类型",
                            placeholder: "例如 skill、model、api",
                            required: true,
                            value: kind(),
                            oninput: move |event: FormEvent| kind.set(event.value()),
                        }
                    }
                    label {
                        span { "生命周期状态" }
                        select {
                            class: "aio-input",
                            aria_label: "资产状态",
                            value: status(),
                            onchange: move |event: FormEvent| status.set(event.value()),
                            option { value: "active", "有效" }
                            option { value: "inactive", "停用" }
                            option { value: "archived", "归档" }
                        }
                    }
                    label {
                        span { "资产来源" }
                        Input {
                            class: "aio-input",
                            aria_label: "资产来源",
                            required: true,
                            value: source(),
                            oninput: move |event: FormEvent| source.set(event.value()),
                        }
                    }
                }
                if let Some(message) = error() {
                    div { class: "aio-asset-form__error", role: "alert", "{message}" }
                }
                footer { class: "aio-asset-dialog__actions",
                    Button {
                        r#type: "button",
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| dialog.set(None),
                        "取消"
                    }
                    Button { r#type: "submit", disabled: saving(),
                        if saving() { "保存中" } else { "保存资产" }
                    }
                }
            }
        }
    }
}

fn asset_form_key(target: &AssetFormTarget) -> String {
    match target {
        AssetFormTarget::New => "asset-new".to_string(),
        AssetFormTarget::Edit(asset) => format!("asset-edit:{}", asset.id),
        AssetFormTarget::Import(skill) => format!("asset-import:{}", skill.id),
    }
}
