use std::{collections::BTreeSet, sync::Arc};

use az_config_center_contract::{
    ConfigCenterStatus, ConfigEntrySummary, ConfigEntryUpsertInput, DotfilesConflict,
    DotfilesMonitorStatus, DotfilesWatchedFile, PairingLocalInfo,
};
use az_ui_components::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    data_table::{DataTable, DataTableCellContext, DataTableColumn, DataTableFixed},
    dialog::{Dialog, DialogDescription, DialogTitle},
    input::Input,
    textarea::Textarea,
};
use dioxus::prelude::*;
use icons::{Eye, EyeOff, Pencil, Plus, RefreshCw, Search, X};
use rudi::Singleton;
use az_dioxus_admin_shell::{
    ConventionPageContext, ConventionPageProvider, DynConventionPageProvider,
};
use studio::browser_http::{format_unix_timestamp, get_api, post_api};

const DOTFILES_PAGE_SIZE: usize = 50;

#[derive(Clone, Debug, Default)]
struct Page;

impl ConventionPageProvider for Page {
    fn key(&self) -> &'static str {
        module_path!()
    }

    fn render(&self, context: ConventionPageContext) -> Element {
        rsx! {
            ConfigWorkspace { title: context.page.title }
        }
    }
}

#[Singleton(name = module_path!())]
fn convention_page() -> DynConventionPageProvider {
    Arc::new(Page)
}

#[derive(Clone, Debug, PartialEq)]
enum ConfigEntryDialogTarget {
    New(String),
    Edit(ConfigEntrySummary),
}

#[derive(Clone, Debug, PartialEq)]
struct ConfigNotice {
    success: bool,
    text: String,
}

impl ConfigNotice {
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
fn ConfigWorkspace(title: String) -> Element {
    let mut runtime_generation = use_signal(|| 0_u64);
    let mut entry_generation = use_signal(|| 0_u64);
    let mut namespace_input = use_signal(|| "az-aio".to_string());
    let mut namespace_query = use_signal(|| "az-aio".to_string());
    let mut entry_search = use_signal(String::new);
    let mut revealed_ids = use_signal(BTreeSet::<String>::new);
    let mut entry_dialog = use_signal(|| None::<ConfigEntryDialogTarget>);
    let mut dotfiles_dialog_open = use_signal(|| false);
    let mut notice = use_signal(|| None::<ConfigNotice>);
    let runtime_status = use_resource(move || {
        let _generation = runtime_generation();
        async move { get_api::<ConfigCenterStatus>("", "/api/config-center/status").await }
    });
    let dotfiles_status = use_resource(move || {
        let _generation = runtime_generation();
        async move {
            get_api::<DotfilesMonitorStatus>("", "/api/config-center/dotfiles")
                .await
                .map(Arc::new)
        }
    });
    let pairing_info = use_resource(move || {
        let _generation = runtime_generation();
        async move { get_api::<PairingLocalInfo>("", "/api/config-center/pairing").await }
    });
    let config_entries = use_resource(move || {
        let _generation = entry_generation();
        let namespace = namespace_query();
        async move {
            let path = format!(
                "/api/config-center/entries?namespace={}",
                urlencoding::encode(&namespace)
            );
            get_api::<Vec<ConfigEntrySummary>>("", &path).await
        }
    });
    let runtime_result = runtime_status.read().as_ref().cloned();
    let dotfiles_result = dotfiles_status.read().as_ref().cloned();
    let pairing_result = pairing_info.read().as_ref().cloned();
    let entry_result = config_entries.read().as_ref().cloned();
    let entry_rows = entry_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let normalized_search = entry_search().trim().to_lowercase();
    let visible_entries = entry_rows
        .iter()
        .filter(|entry| {
            normalized_search.is_empty()
                || entry.key.to_lowercase().contains(&normalized_search)
                || entry.namespace.to_lowercase().contains(&normalized_search)
        })
        .cloned()
        .collect::<Vec<_>>();
    let entry_empty_text = match entry_result.as_ref() {
        None => "正在加载配置条目".to_string(),
        Some(Err(error)) => error.clone(),
        Some(Ok(_)) if !normalized_search.is_empty() => "没有匹配的配置条目".to_string(),
        Some(Ok(_)) => "当前命名空间暂无配置".to_string(),
    };
    let loaded_dotfiles = dotfiles_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned();

    rsx! {
        section { class: "aio-config-workbench",
            header { class: "aio-config-workbench__header",
                div {
                    h2 { "{title}" }
                    p { "PostgreSQL 配置、Dotfiles 巡检与设备配对" }
                }
                Button {
                    r#type: "button",
                    size: ButtonSize::IconSm,
                    variant: ButtonVariant::Outline,
                    title: "刷新配置中心",
                    aria_label: "刷新配置中心",
                    onclick: move |_| {
                        notice.set(None);
                        runtime_generation.with_mut(|value| {
                            *value = value.saturating_add(1);
                        });
                        entry_generation.with_mut(|value| {
                            *value = value.saturating_add(1);
                        });
                    },
                    RefreshCw { class: "size-4" }
                }
            }
            if let Some(message) = notice() {
                div {
                    class: if message.success {
                        "aio-config-notice"
                    } else {
                        "aio-config-notice is-error"
                    },
                    role: if message.success { "status" } else { "alert" },
                    "{message.text}"
                }
            }
            div { class: "aio-config-runtime",
                section { class: "aio-config-runtime__section",
                    header {
                        strong { "PostgreSQL" }
                        match runtime_result.as_ref() {
                            Some(Ok(status)) if status.store_connected => rsx! {
                                Badge { variant: BadgeVariant::Secondary, "已连接" }
                            },
                            Some(Ok(_)) => rsx! {
                                Badge { variant: BadgeVariant::Destructive, "未连接" }
                            },
                            Some(Err(_)) => rsx! {
                                Badge { variant: BadgeVariant::Destructive, "读取失败" }
                            },
                            None => rsx! { Badge { variant: BadgeVariant::Outline, "加载中" } },
                        }
                    }
                    if let Some(Ok(status)) = runtime_result.as_ref() {
                        code { "{status.table_prefix}" }
                        span { if status.database_configured { "数据库已配置" } else { "数据库未配置" } }
                    } else if let Some(Err(error)) = runtime_result.as_ref() {
                        p { role: "alert", "{error}" }
                    }
                }
                section { class: "aio-config-runtime__section",
                    header {
                        strong { "Dotfiles" }
                        if let Some(status) = loaded_dotfiles.as_ref() {
                            Badge {
                                variant: if status.conflict_files > 0 {
                                    BadgeVariant::Destructive
                                } else if status.changed_files > 0 {
                                    BadgeVariant::Outline
                                } else {
                                    BadgeVariant::Secondary
                                },
                                if status.conflict_files > 0 {
                                    "存在冲突"
                                } else if status.changed_files > 0 {
                                    "待同步"
                                } else {
                                    "已对齐"
                                }
                            }
                        } else {
                            Badge { variant: BadgeVariant::Outline, "扫描中" }
                        }
                    }
                    if let Some(status) = loaded_dotfiles.as_ref() {
                        div { class: "aio-config-runtime__metrics",
                            span { strong { "{status.watched_files}" } "监控" }
                            span { strong { "{status.changed_files}" } "变更" }
                            span { strong { "{status.conflict_files}" } "冲突" }
                        }
                        Button {
                            r#type: "button",
                            size: ButtonSize::Xs,
                            variant: ButtonVariant::Outline,
                            onclick: move |_| dotfiles_dialog_open.set(true),
                            "查看巡检"
                        }
                    } else if let Some(Err(error)) = dotfiles_result.as_ref() {
                        p { role: "alert", "{error}" }
                    }
                }
                section { class: "aio-config-runtime__section",
                    header {
                        strong { "本机配对" }
                        Badge { variant: BadgeVariant::Outline, "设备身份" }
                    }
                    if let Some(Ok(info)) = pairing_result.as_ref() {
                        span { "{info.device_name}" }
                        code { title: "{info.fingerprint}", "{info.fingerprint}" }
                    } else if let Some(Err(error)) = pairing_result.as_ref() {
                        p { role: "alert", "{error}" }
                    } else {
                        span { "正在读取设备身份" }
                    }
                }
                section { class: "aio-config-runtime__section aio-config-runtime__section--paths",
                    header {
                        strong { "运行目录" }
                        Badge { variant: BadgeVariant::Outline, "XDG" }
                    }
                    if let Some(Ok(status)) = runtime_result.as_ref() {
                        dl {
                            div { dt { "配置" } dd { code { title: "{status.paths.config_dir}", "{status.paths.config_dir}" } } }
                            div { dt { "数据" } dd { code { title: "{status.paths.data_dir}", "{status.paths.data_dir}" } } }
                        }
                    }
                }
            }
            section { class: "aio-config-entries",
                header { class: "aio-config-entries__header",
                    div {
                        h3 { "命名空间配置" }
                        span { "{visible_entries.len()} / {entry_rows.len()}" }
                    }
                    Button {
                        r#type: "button",
                        onclick: move |_| {
                            notice.set(None);
                            entry_dialog.set(Some(ConfigEntryDialogTarget::New(namespace_query())));
                        },
                        Plus { class: "size-4" }
                        "新增配置"
                    }
                }
                div { class: "aio-config-toolbar",
                    form {
                        class: "aio-config-namespace",
                        onsubmit: move |event| {
                            event.prevent_default();
                            let namespace = namespace_input().trim().to_string();
                            let namespace = if namespace.is_empty() {
                                "az-aio".to_string()
                            } else {
                                namespace
                            };
                            namespace_input.set(namespace.clone());
                            namespace_query.set(namespace);
                            entry_search.set(String::new());
                            revealed_ids.set(BTreeSet::new());
                            entry_generation.with_mut(|value| {
                                *value = value.saturating_add(1);
                            });
                        },
                        Input {
                            class: "aio-input",
                            aria_label: "配置命名空间",
                            placeholder: "命名空间",
                            value: namespace_input(),
                            oninput: move |event: FormEvent| namespace_input.set(event.value()),
                        }
                        Button { r#type: "submit", size: ButtonSize::Sm, "加载" }
                    }
                    div { class: "aio-config-search",
                        Search { class: "size-4" }
                        Input {
                            class: "aio-input",
                            aria_label: "搜索配置键",
                            placeholder: "搜索配置键",
                            value: entry_search(),
                            oninput: move |event: FormEvent| entry_search.set(event.value()),
                        }
                    }
                }
                DataTable::<ConfigEntrySummary> {
                    key: "config-entries:{namespace_query()}",
                    class: "aio-config-data-table",
                    aria_label: "配置条目表".to_string(),
                    rows: visible_entries,
                    columns: config_entry_columns(),
                    max_height: "clamp(16rem, calc(100vh - 31rem), 34rem)",
                    empty_text: entry_empty_text,
                    row_key: |row: ConfigEntrySummary| row.id.clone(),
                    render_cell: move |cell: DataTableCellContext<ConfigEntrySummary>| {
                        config_entry_cell(cell, revealed_ids, entry_dialog, notice)
                    },
                }
            }
        }
        if let Some(target) = entry_dialog() {
            ConfigEntryDialog {
                key: "{config_entry_dialog_key(&target)}",
                target,
                entry_dialog,
                notice,
                entry_generation,
            }
        }
        if dotfiles_dialog_open()
            && let Some(status) = loaded_dotfiles
        {
            DotfilesDialog { status, dotfiles_dialog_open }
        }
    }
}

fn config_entry_columns() -> Vec<DataTableColumn> {
    vec![
        DataTableColumn::leaf("key", "配置键")
            .width(240)
            .fixed(DataTableFixed::Left),
        DataTableColumn::leaf("namespace", "命名空间").width(144),
        DataTableColumn::leaf("value", "配置值").width(280),
        DataTableColumn::leaf("updated_at", "更新时间").width(168),
        DataTableColumn::leaf("actions", "操作")
            .width(96)
            .fixed(DataTableFixed::Right),
    ]
}

fn config_entry_cell(
    cell: DataTableCellContext<ConfigEntrySummary>,
    mut revealed_ids: Signal<BTreeSet<String>>,
    mut entry_dialog: Signal<Option<ConfigEntryDialogTarget>>,
    mut notice: Signal<Option<ConfigNotice>>,
) -> Element {
    let row = cell.row;
    match cell.column.key.as_str() {
        "key" => rsx! {
            div { class: "aio-config-entry-key",
                strong { "{row.key}" }
                code { "{row.id}" }
            }
        },
        "namespace" => rsx! { code { "{row.namespace}" } },
        "value" => {
            let revealed = revealed_ids().contains(&row.id);
            let row_id = row.id.clone();
            rsx! {
                div { class: "aio-config-secret",
                    code {
                        class: if revealed { "is-revealed" } else { "" },
                        if revealed { "{row.value}" } else { "********" }
                    }
                    Button {
                        r#type: "button",
                        size: ButtonSize::IconXs,
                        variant: ButtonVariant::Ghost,
                        title: if revealed { "隐藏配置值" } else { "显示配置值" },
                        aria_label: if revealed {
                            format!("隐藏配置值 {}", row.key)
                        } else {
                            format!("显示配置值 {}", row.key)
                        },
                        onclick: move |_| {
                            revealed_ids.with_mut(|ids| {
                                if revealed {
                                    ids.remove(&row_id);
                                } else {
                                    ids.insert(row_id.clone());
                                }
                            });
                        },
                        if revealed {
                            EyeOff { class: "size-4" }
                        } else {
                            Eye { class: "size-4" }
                        }
                    }
                }
            }
        }
        "updated_at" => rsx! { "{format_unix_timestamp(&row.updated_at)}" },
        "actions" => {
            let row_for_edit = row.clone();
            rsx! {
                div { class: "aio-config-row-actions",
                    Button {
                        r#type: "button",
                        size: ButtonSize::IconSm,
                        variant: ButtonVariant::Ghost,
                        title: "编辑配置",
                        aria_label: "编辑配置 {row.key}",
                        onclick: move |_| {
                            notice.set(None);
                            entry_dialog.set(Some(ConfigEntryDialogTarget::Edit(
                                row_for_edit.clone(),
                            )));
                        },
                        Pencil { class: "size-4" }
                    }
                }
            }
        }
        _ => rsx! { "" },
    }
}

#[component]
fn ConfigEntryDialog(
    target: ConfigEntryDialogTarget,
    mut entry_dialog: Signal<Option<ConfigEntryDialogTarget>>,
    mut notice: Signal<Option<ConfigNotice>>,
    mut entry_generation: Signal<u64>,
) -> Element {
    let editing = matches!(&target, ConfigEntryDialogTarget::Edit(_));
    let (id, initial_namespace, initial_key, initial_value) = match &target {
        ConfigEntryDialogTarget::New(namespace) => (
            None,
            namespace.clone(),
            String::new(),
            String::new(),
        ),
        ConfigEntryDialogTarget::Edit(entry) => (
            Some(entry.id.clone()),
            entry.namespace.clone(),
            entry.key.clone(),
            entry.value.clone(),
        ),
    };
    let mut namespace = use_signal(move || initial_namespace);
    let mut key = use_signal(move || initial_key);
    let mut value = use_signal(move || initial_value);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        Dialog {
            class: "aio-config-dialog",
            open: true,
            on_open_change: move |open: bool| {
                if !open {
                    entry_dialog.set(None);
                }
            },
            header { class: "aio-config-dialog__header",
                div {
                    DialogTitle { if editing { "编辑配置" } else { "新增配置" } }
                    DialogDescription { "配置值只在此编辑窗口和明确显示操作中可见" }
                }
                Button {
                    r#type: "button",
                    size: ButtonSize::IconSm,
                    variant: ButtonVariant::Ghost,
                    title: "关闭配置编辑",
                    aria_label: "关闭配置编辑",
                    onclick: move |_| entry_dialog.set(None),
                    X { class: "size-4" }
                }
            }
            form {
                class: "aio-config-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    let normalized_namespace = namespace().trim().to_string();
                    let normalized_key = key().trim().to_string();
                    let normalized_value = value().trim().to_string();
                    if normalized_namespace.is_empty()
                        || normalized_key.is_empty()
                        || normalized_value.is_empty()
                    {
                        error.set(Some("命名空间、配置键和值不能为空".to_string()));
                        return;
                    }
                    let input = ConfigEntryUpsertInput {
                        id: id.clone(),
                        namespace: normalized_namespace,
                        key: normalized_key,
                        value: normalized_value,
                    };
                    saving.set(true);
                    error.set(None);
                    notice.set(None);
                    spawn(async move {
                        match post_api::<_, ConfigEntrySummary>(
                            "",
                            "/api/config-center/entry",
                            &input,
                        )
                        .await
                        {
                            Ok(_) => {
                                entry_generation.with_mut(|value| {
                                    *value = value.saturating_add(1);
                                });
                                notice.set(Some(ConfigNotice::success(if editing {
                                    "配置已更新"
                                } else {
                                    "配置已创建"
                                })));
                                entry_dialog.set(None);
                            }
                            Err(message) => {
                                saving.set(false);
                                error.set(Some(message.clone()));
                                notice.set(Some(ConfigNotice::error(message)));
                            }
                        }
                    });
                },
                div { class: "aio-config-form__grid",
                    label {
                        span { "命名空间" }
                        Input {
                            class: "aio-input",
                            aria_label: "条目命名空间",
                            required: true,
                            value: namespace(),
                            oninput: move |event: FormEvent| namespace.set(event.value()),
                        }
                    }
                    label {
                        span { "配置键" }
                        Input {
                            class: "aio-input",
                            aria_label: "配置键",
                            placeholder: "例如 studio.theme",
                            required: true,
                            value: key(),
                            oninput: move |event: FormEvent| key.set(event.value()),
                        }
                    }
                    label { class: "aio-config-form__wide",
                        span { "配置值" }
                        Textarea {
                            class: "aio-input aio-config-form__value",
                            aria_label: "配置值",
                            autocomplete: "off",
                            rows: "6",
                            required: true,
                            value: value(),
                            oninput: move |event: FormEvent| value.set(event.value()),
                        }
                    }
                }
                if let Some(message) = error() {
                    div { class: "aio-config-form__error", role: "alert", "{message}" }
                }
                footer { class: "aio-config-dialog__actions",
                    Button {
                        r#type: "button",
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| entry_dialog.set(None),
                        "取消"
                    }
                    Button { r#type: "submit", disabled: saving(),
                        if saving() { "保存中" } else { "保存配置" }
                    }
                }
            }
        }
    }
}

#[component]
fn DotfilesDialog(
    status: Arc<DotfilesMonitorStatus>,
    mut dotfiles_dialog_open: Signal<bool>,
) -> Element {
    let mut search = use_signal(String::new);
    let mut visible_limit = use_signal(|| DOTFILES_PAGE_SIZE);
    let normalized_search = search().trim().to_lowercase();
    let matching_files = status
        .pending_files
        .iter()
        .filter(|file| {
            normalized_search.is_empty()
                || file.relative_path.to_lowercase().contains(&normalized_search)
                || file.target_name.to_lowercase().contains(&normalized_search)
                || file.status.to_lowercase().contains(&normalized_search)
        })
        .cloned()
        .collect::<Vec<_>>();
    let total_matching = matching_files.len();
    let pending_rows = matching_files
        .into_iter()
        .take(visible_limit())
        .collect::<Vec<_>>();
    let conflict_rows = status
        .conflicts
        .iter()
        .take(DOTFILES_PAGE_SIZE)
        .cloned()
        .collect::<Vec<_>>();
    let can_show_more = visible_limit() < total_matching;

    rsx! {
        Dialog {
            class: "aio-config-dialog aio-config-dialog--dotfiles",
            open: true,
            on_open_change: move |open: bool| {
                if !open {
                    dotfiles_dialog_open.set(false);
                }
            },
            header { class: "aio-config-dialog__header",
                div {
                    DialogTitle { "Dotfiles 巡检" }
                    DialogDescription {
                        "{status.watched_files} 个文件 · {status.changed_files} 个变更 · {status.conflict_files} 个冲突"
                    }
                }
                Button {
                    r#type: "button",
                    size: ButtonSize::IconSm,
                    variant: ButtonVariant::Ghost,
                    title: "关闭 Dotfiles 巡检",
                    aria_label: "关闭 Dotfiles 巡检",
                    onclick: move |_| dotfiles_dialog_open.set(false),
                    X { class: "size-4" }
                }
            }
            div { class: "aio-dotfiles-dialog__body",
                dl { class: "aio-dotfiles-paths",
                    div { dt { "仓库" } dd { code { title: "{status.root}", "{status.root}" } } }
                    div { dt { "源目录" } dd { code { title: "{status.source_home}", "{status.source_home}" } } }
                    div { dt { "基线" } dd { code { title: "{status.baseline_path}", "{status.baseline_path}" } } }
                    div { dt { "更新时间" } dd { "{format_unix_timestamp(&status.updated_at)}" } }
                }
                section { class: "aio-dotfiles-section",
                    header {
                        div {
                            h3 { "待处理文件" }
                            span { "{pending_rows.len()} / {total_matching}" }
                        }
                        div { class: "aio-config-search",
                            Search { class: "size-4" }
                            Input {
                                class: "aio-input",
                                aria_label: "搜索待处理文件",
                                placeholder: "路径、设备或状态",
                                value: search(),
                                oninput: move |event: FormEvent| {
                                    search.set(event.value());
                                    visible_limit.set(DOTFILES_PAGE_SIZE);
                                },
                            }
                        }
                    }
                    DataTable::<DotfilesWatchedFile> {
                        class: "aio-config-data-table",
                        aria_label: "Dotfiles 待处理文件表".to_string(),
                        rows: pending_rows,
                        columns: dotfiles_file_columns(),
                        max_height: "15rem",
                        empty_text: "暂无待处理文件".to_string(),
                        row_key: |row: DotfilesWatchedFile| {
                            format!("{}:{}", row.target_name, row.relative_path)
                        },
                        render_cell: dotfiles_file_cell,
                    }
                    if can_show_more {
                        Button {
                            r#type: "button",
                            size: ButtonSize::Sm,
                            variant: ButtonVariant::Outline,
                            onclick: move |_| {
                                visible_limit.with_mut(|value| {
                                    *value = value.saturating_add(DOTFILES_PAGE_SIZE);
                                });
                            },
                            "继续显示 {DOTFILES_PAGE_SIZE} 条"
                        }
                    }
                }
                section { class: "aio-dotfiles-section",
                    header {
                        div {
                            h3 { "冲突" }
                            span { "{status.conflicts.len()}" }
                        }
                    }
                    DataTable::<DotfilesConflict> {
                        class: "aio-config-data-table",
                        aria_label: "Dotfiles 冲突表".to_string(),
                        rows: conflict_rows,
                        columns: dotfiles_conflict_columns(),
                        max_height: "12rem",
                        empty_text: "暂无冲突".to_string(),
                        row_key: |row: DotfilesConflict| row.id.clone(),
                        render_cell: dotfiles_conflict_cell,
                    }
                }
            }
        }
    }
}

fn dotfiles_file_columns() -> Vec<DataTableColumn> {
    vec![
        DataTableColumn::leaf("relative_path", "相对路径")
            .width(260)
            .fixed(DataTableFixed::Left),
        DataTableColumn::leaf("target_name", "目标").width(128),
        DataTableColumn::leaf("status", "状态").width(120),
        DataTableColumn::leaf("detail", "说明").width(420),
    ]
}

fn dotfiles_conflict_columns() -> Vec<DataTableColumn> {
    vec![
        DataTableColumn::leaf("title", "冲突")
            .width(240)
            .fixed(DataTableFixed::Left),
        DataTableColumn::leaf("relative_path", "相对路径").width(240),
        DataTableColumn::leaf("risk", "风险").width(120),
        DataTableColumn::leaf("suggestion", "建议").width(420),
    ]
}

fn dotfiles_file_cell(cell: DataTableCellContext<DotfilesWatchedFile>) -> Element {
    let row = cell.row;
    match cell.column.key.as_str() {
        "relative_path" => rsx! { code { "{row.relative_path}" } },
        "target_name" => rsx! { "{row.target_name}" },
        "status" => rsx! {
            Badge { variant: BadgeVariant::Outline, "{dotfiles_status_label(&row.status)}" }
        },
        "detail" => rsx! { span { title: "{row.detail}", "{row.detail}" } },
        _ => rsx! { "" },
    }
}

fn dotfiles_conflict_cell(cell: DataTableCellContext<DotfilesConflict>) -> Element {
    let row = cell.row;
    match cell.column.key.as_str() {
        "title" => rsx! { strong { "{row.title}" } },
        "relative_path" => rsx! { code { "{row.relative_path}" } },
        "risk" => rsx! { Badge { variant: BadgeVariant::Destructive, "{row.risk}" } },
        "suggestion" => rsx! { span { title: "{row.suggestion}", "{row.suggestion}" } },
        _ => rsx! { "" },
    }
}

fn dotfiles_status_label(status: &str) -> &str {
    match status {
        "baseline" => "建立基线",
        "one-sided" => "单侧变更",
        "mergeable" => "可合并",
        "line-conflict" => "行冲突",
        "same" => "已对齐",
        _ => status,
    }
}

fn config_entry_dialog_key(target: &ConfigEntryDialogTarget) -> String {
    match target {
        ConfigEntryDialogTarget::New(namespace) => format!("config-new:{namespace}"),
        ConfigEntryDialogTarget::Edit(entry) => format!("config-edit:{}", entry.id),
    }
}
