#![allow(non_snake_case)]

//! SSH 服务器运维 SSR 页面。

use std::time::{SystemTime, UNIX_EPOCH};

use az_aio_platform::plugin::contract::NativeRenderContext;
use dioxus::prelude::*;
use registry::ui::{
    badge::Badge,
    button::Button,
    card::{Card, CardContent, CardDescription, CardHeader, CardTitle},
    table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow},
};

use crate::{
    contract::{
        AUTH_PASSWORD_ENV, AUTH_PRIVATE_KEY, COMMAND_KIND_MONITOR, COMMAND_MODEL, RESULT_MODEL,
        STATUS_FAILED, STATUS_SUCCESS, STATUS_UNSUPPORTED, SshCommandResultView, SshCommandView,
        SshDashboardSnapshot, SshTargetView, TARGET_MODEL, UI_ACTION_PATH,
    },
    state::{run_ssh_future, service},
};

struct PageSnapshot {
    dashboard: SshDashboardSnapshot,
    error: Option<String>,
}

/// 渲染 SSH 目标、低代码命令和执行结果工作台。
pub fn SshOperationsPage(context: NativeRenderContext) -> Element {
    let snapshot = load_snapshot();
    let view = query_value(&context.active_route, "view").unwrap_or_else(|| "overview".to_string());
    let message = query_value(&context.active_route, "message");
    let route_error = query_value(&context.active_route, "error");

    rsx! {
        div { class: "space-y-5",
            Card {
                CardHeader {
                    div { class: "flex flex-wrap items-start justify-between gap-4",
                        div {
                            CardTitle { "SSH 服务器运维" }
                            CardDescription { "真实主机直连 · 低代码命令 · 跨硬件自动探测" }
                        }
                        div { class: "flex flex-wrap gap-2",
                            Badge { "EngineStore" }
                            Badge { "libssh2" }
                            Badge { "只读内置命令" }
                        }
                    }
                }
            }

            if let Some(message) = message {
                div { class: "rounded-md border border-green-600/30 bg-green-600/10 p-3 text-sm text-green-800", "{message}" }
            }
            if let Some(error) = route_error.or(snapshot.error.clone()) {
                div { class: "rounded-md border border-destructive bg-destructive/10 p-3 text-sm text-destructive", "{error}" }
            }

            if !snapshot.dashboard.template_ready {
                {render_template_empty()}
            } else {
                match view.as_str() {
                    "targets" => render_targets(&snapshot.dashboard),
                    "commands" => render_commands(&snapshot.dashboard),
                    "results" => render_results(&snapshot.dashboard),
                    _ => render_overview(&snapshot.dashboard),
                }
            }
        }
    }
}

fn load_snapshot() -> PageSnapshot {
    match service().and_then(|service| run_ssh_future(async move { service.dashboard().await })) {
        Ok(dashboard) => PageSnapshot {
            dashboard,
            error: None,
        },
        Err(error) => PageSnapshot {
            dashboard: SshDashboardSnapshot::default(),
            error: Some(error.to_string()),
        },
    }
}

fn render_template_empty() -> Element {
    rsx! {
        Card {
            CardHeader {
                CardTitle { "初始化 SSH 低代码模板" }
                CardDescription { "将在共享 PostgreSQL 中创建目标、命令和最近执行结果模型，并写入跨硬件只读命令。" }
            }
            CardContent {
                form { method: "post", action: UI_ACTION_PATH, class: "flex flex-wrap items-center gap-4",
                    input { r#type: "hidden", name: "action", value: "apply_template" }
                    input { r#type: "hidden", name: "return_view", value: "overview" }
                    label { class: "flex items-center gap-2 text-sm",
                        input { r#type: "checkbox", name: "seed_builtin_commands", value: "true", checked: true }
                        span { "写入内置硬件监测命令" }
                    }
                    Button { button_type: "submit", "初始化服务器运维" }
                }
            }
        }
    }
}

fn render_overview(snapshot: &SshDashboardSnapshot) -> Element {
    let enabled_targets = snapshot
        .targets
        .iter()
        .filter(|target| target.enabled)
        .count();
    let enabled_commands = snapshot
        .commands
        .iter()
        .filter(|command| command.enabled)
        .count();
    let success = snapshot
        .results
        .iter()
        .filter(|result| result.status == STATUS_SUCCESS)
        .count();
    let failed = snapshot
        .results
        .iter()
        .filter(|result| result.status == STATUS_FAILED)
        .count();

    rsx! {
        div { class: "grid gap-3 sm:grid-cols-2 xl:grid-cols-4",
            Metric { label: "已启用目标", value: enabled_targets.to_string(), tone: "blue" }
            Metric { label: "已启用命令", value: enabled_commands.to_string(), tone: "slate" }
            Metric { label: "最近成功", value: success.to_string(), tone: "green" }
            Metric { label: "最近异常", value: failed.to_string(), tone: "red" }
        }

        div { class: "space-y-3",
            div { class: "flex flex-wrap items-center justify-between gap-3",
                div {
                    h2 { class: "text-base font-semibold", "SSH 目标" }
                    p { class: "text-sm text-muted-foreground", "采集会自动跳过不匹配当前硬件的命令。" }
                }
                a { class: "text-sm font-medium text-primary hover:underline", href: page_href("targets"), "管理目标" }
            }
            if snapshot.targets.is_empty() {
                div { class: "rounded-md border border-dashed p-6 text-sm text-muted-foreground", "还没有 SSH 目标。" }
            } else {
                div { class: "grid gap-3 lg:grid-cols-2 2xl:grid-cols-3",
                    for target in &snapshot.targets {
                        {target_summary(target, &snapshot.results)}
                    }
                }
            }
        }

        {recent_results(snapshot)}
    }
}

#[component]
fn Metric(label: &'static str, value: String, tone: &'static str) -> Element {
    let class = match tone {
        "green" => "border-green-500/30 bg-green-500/10",
        "red" => "border-red-500/30 bg-red-500/10",
        "blue" => "border-blue-500/30 bg-blue-500/10",
        _ => "border-slate-300 bg-white",
    };
    rsx! {
        div { class: "rounded-md border p-4 {class}",
            div { class: "text-xs text-muted-foreground", "{label}" }
            div { class: "mt-1 text-2xl font-semibold", "{value}" }
        }
    }
}

fn target_summary(target: &SshTargetView, results: &[SshCommandResultView]) -> Element {
    let (status, status_class) = target_status(&target.code, results);
    let address = format!("{}@{}:{}", target.username, target.host, target.port);
    let target_code = target.code.clone();
    rsx! {
        div { class: "rounded-md border bg-card p-4",
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    h3 { class: "truncate font-semibold", "{target.name}" }
                    code { class: "text-xs text-muted-foreground", "{address}" }
                }
                span { class: "shrink-0 rounded-full px-2 py-1 text-xs font-medium {status_class}", "{status}" }
            }
            p { class: "mt-3 min-h-5 text-sm text-muted-foreground", "{target.description}" }
            div { class: "mt-4 flex items-center justify-between gap-3 border-t pt-3",
                span { class: "text-xs text-muted-foreground", "认证：{auth_label(&target.auth_type)}" }
                if target.enabled {
                    form { method: "post", action: UI_ACTION_PATH,
                        input { r#type: "hidden", name: "action", value: "collect" }
                        input { r#type: "hidden", name: "return_view", value: "overview" }
                        input { r#type: "hidden", name: "target_code", value: target_code }
                        Button { button_type: "submit", "立即采集" }
                    }
                } else {
                    span { class: "text-xs text-muted-foreground", "已禁用" }
                }
            }
        }
    }
}

fn render_targets(snapshot: &SshDashboardSnapshot) -> Element {
    let lowcode_href = lowcode_records_href(TARGET_MODEL);
    rsx! {
        Card {
            CardHeader {
                div { class: "flex flex-wrap items-start justify-between gap-3",
                    div {
                        CardTitle { "保存 SSH 目标" }
                        CardDescription { "使用真实主机名或 IP；密码和私钥口令只引用 AIO 进程环境变量。" }
                    }
                    a { class: "text-sm font-medium text-primary hover:underline", href: lowcode_href, "打开低代码记录" }
                }
            }
            CardContent {
                form { method: "post", action: UI_ACTION_PATH, class: "grid gap-4 lg:grid-cols-4",
                    input { r#type: "hidden", name: "action", value: "upsert_target" }
                    input { r#type: "hidden", name: "return_view", value: "targets" }
                    FieldLabel { title: "目标编码", required: true,
                        input { class: "az-input", name: "code", placeholder: "gpu-server-01", required: true }
                    }
                    FieldLabel { title: "显示名称", required: true,
                        input { class: "az-input", name: "name", placeholder: "天津 AI 服务器", required: true }
                    }
                    FieldLabel { title: "主机名或 IP", required: true,
                        input { class: "az-input", name: "host", placeholder: "192.168.31.100", required: true }
                    }
                    FieldLabel { title: "SSH 端口", required: true,
                        input { class: "az-input", r#type: "number", name: "port", value: "22", min: "1", max: "65535", required: true }
                    }
                    FieldLabel { title: "登录用户", required: true,
                        input { class: "az-input", name: "username", placeholder: "ubuntu", required: true }
                    }
                    FieldLabel { title: "认证方式", required: true,
                        select { class: "az-input", name: "auth_type", required: true,
                            option { value: AUTH_PRIVATE_KEY, "私钥文件" }
                            option { value: AUTH_PASSWORD_ENV, "密码环境变量" }
                        }
                    }
                    FieldLabel { title: "私钥路径",
                        input { class: "az-input", name: "private_key_path", value: "~/.ssh/id_ed25519", placeholder: "~/.ssh/id_ed25519" }
                    }
                    FieldLabel { title: "密码环境变量",
                        input { class: "az-input", name: "password_env", placeholder: "SSH_GPU_SERVER_PASSWORD" }
                    }
                    FieldLabel { title: "私钥口令环境变量",
                        input { class: "az-input", name: "passphrase_env", placeholder: "SSH_KEY_PASSPHRASE" }
                    }
                    FieldLabel { title: "备注",
                        input { class: "az-input", name: "description", placeholder: "机型、机房或用途" }
                    }
                    label { class: "flex items-center gap-2 self-end pb-2 text-sm",
                        input { r#type: "checkbox", name: "enabled", value: "true", checked: true }
                        span { "启用目标" }
                    }
                    div { class: "flex items-end justify-end lg:col-span-4",
                        Button { button_type: "submit", "保存目标" }
                    }
                }
            }
        }

        Card {
            CardHeader {
                CardTitle { "目标列表" }
                CardDescription { "相同目标编码会直接更新现有记录。" }
            }
            CardContent {
                Table {
                    TableHeader { TableRow {
                        TableHead { "名称" }
                        TableHead { "连接地址" }
                        TableHead { "认证" }
                        TableHead { "状态" }
                    } }
                    TableBody {
                        for target in &snapshot.targets {
                            TableRow {
                                TableCell {
                                    div { class: "font-medium", "{target.name}" }
                                    code { class: "text-xs text-muted-foreground", "{target.code}" }
                                }
                                TableCell { code { "{target.username}@{target.host}:{target.port}" } }
                                TableCell { "{auth_label(&target.auth_type)}" }
                                TableCell { if target.enabled { "启用" } else { "禁用" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_commands(snapshot: &SshDashboardSnapshot) -> Element {
    let lowcode_href = lowcode_records_href(COMMAND_MODEL);
    rsx! {
        Card {
            CardHeader {
                div { class: "flex flex-wrap items-start justify-between gap-3",
                    div {
                        CardTitle { "配置监测命令" }
                        CardDescription { "探测脚本退出码为 0 才执行命令；留空表示适用于所有目标。" }
                    }
                    a { class: "text-sm font-medium text-primary hover:underline", href: lowcode_href, "打开低代码记录" }
                }
            }
            CardContent {
                form { method: "post", action: UI_ACTION_PATH, class: "grid gap-4 lg:grid-cols-4",
                    input { r#type: "hidden", name: "action", value: "upsert_command" }
                    input { r#type: "hidden", name: "return_view", value: "commands" }
                    FieldLabel { title: "命令编码", required: true,
                        input { class: "az-input", name: "code", placeholder: "custom-accelerator", required: true }
                    }
                    FieldLabel { title: "命令名称", required: true,
                        input { class: "az-input", name: "name", placeholder: "加速卡状态", required: true }
                    }
                    FieldLabel { title: "分类", required: true,
                        input { class: "az-input", name: "category", placeholder: "加速卡", required: true }
                    }
                    FieldLabel { title: "硬件族", required: true,
                        input { class: "az-input", name: "hardware_family", value: "generic", placeholder: "generic", required: true }
                    }
                    FieldLabel { title: "命令类型", required: true,
                        select { class: "az-input", name: "kind", required: true,
                            option { value: COMMAND_KIND_MONITOR, "监测命令" }
                            option { value: "operation", "运维操作" }
                        }
                    }
                    FieldLabel { title: "超时秒数", required: true,
                        input { class: "az-input", r#type: "number", name: "timeout_secs", value: "15", min: "1", max: "300", required: true }
                    }
                    FieldLabel { title: "排序", required: true,
                        input { class: "az-input", r#type: "number", name: "order_index", value: "200", required: true }
                    }
                    label { class: "flex items-center gap-2 self-end pb-2 text-sm",
                        input { r#type: "checkbox", name: "enabled", value: "true", checked: true }
                        span { "启用命令" }
                    }
                    FieldLabel { title: "适配探测脚本",
                        textarea { class: "az-input min-h-24 font-mono", name: "detect_script", placeholder: "command -v hy-smi >/dev/null 2>&1" }
                    }
                    FieldLabel { title: "执行脚本", required: true,
                        textarea { class: "az-input min-h-24 font-mono", name: "command_script", placeholder: "hy-smi --showallinfo", required: true }
                    }
                    div { class: "flex items-end justify-end lg:col-span-4",
                        Button { button_type: "submit", "保存命令" }
                    }
                }
            }
        }

        Card {
            CardHeader {
                CardTitle { "命令目录" }
                CardDescription { "“采集全部”只运行监测命令；运维操作必须在这里明确选择目标后单独执行。" }
            }
            CardContent {
                Table {
                    TableHeader { TableRow {
                        TableHead { "命令" }
                        TableHead { "适配" }
                        TableHead { "类型" }
                        TableHead { "超时" }
                        TableHead { "执行" }
                    } }
                    TableBody {
                        for command in &snapshot.commands {
                            {command_row(command, &snapshot.targets)}
                        }
                    }
                }
            }
        }
    }
}

fn command_row(command: &SshCommandView, targets: &[SshTargetView]) -> Element {
    let command_code = command.code.clone();
    let enabled_targets = targets
        .iter()
        .filter(|target| target.enabled)
        .collect::<Vec<_>>();
    rsx! {
        TableRow {
            TableCell {
                div { class: "font-medium", "{command.name}" }
                code { class: "text-xs text-muted-foreground", "{command.code}" }
                details { class: "mt-2 max-w-xl text-xs",
                    summary { class: "cursor-pointer text-primary", "查看脚本" }
                    if !command.detect_script.is_empty() {
                        div { class: "mt-2 text-muted-foreground", "探测" }
                        pre { class: "mt-1 max-h-40 overflow-auto whitespace-pre-wrap rounded-md bg-muted p-2", "{command.detect_script}" }
                    }
                    div { class: "mt-2 text-muted-foreground", "执行" }
                    pre { class: "mt-1 max-h-40 overflow-auto whitespace-pre-wrap rounded-md bg-muted p-2", "{command.command_script}" }
                }
            }
            TableCell {
                div { "{hardware_label(&command.hardware_family)}" }
                div { class: "text-xs text-muted-foreground", "{command.category}" }
            }
            TableCell { "{kind_label(&command.kind)}" }
            TableCell { "{command.timeout_secs} 秒" }
            TableCell {
                if !command.enabled {
                    span { class: "text-xs text-muted-foreground", "已禁用" }
                } else if enabled_targets.is_empty() {
                    span { class: "text-xs text-muted-foreground", "无可用目标" }
                } else {
                    form { method: "post", action: UI_ACTION_PATH, class: "flex min-w-56 items-center gap-2",
                        input { r#type: "hidden", name: "action", value: "execute" }
                        input { r#type: "hidden", name: "return_view", value: "commands" }
                        input { r#type: "hidden", name: "command_code", value: command_code }
                        select { class: "az-input", name: "target_code", required: true,
                            for target in enabled_targets {
                                option { value: target.code.clone(), "{target.name}" }
                            }
                        }
                        Button { button_type: "submit", "执行" }
                    }
                }
            }
        }
    }
}

fn render_results(snapshot: &SshDashboardSnapshot) -> Element {
    let lowcode_href = lowcode_records_href(RESULT_MODEL);
    rsx! {
        Card {
            CardHeader {
                div { class: "flex flex-wrap items-start justify-between gap-3",
                    div {
                        CardTitle { "最近执行结果" }
                        CardDescription { "每个目标和命令仅保留最近一次结果，输出最多保存 64 KiB。" }
                    }
                    a { class: "text-sm font-medium text-primary hover:underline", href: lowcode_href, "打开低代码记录" }
                }
            }
            CardContent {
                if snapshot.results.is_empty() {
                    div { class: "rounded-md border border-dashed p-6 text-sm text-muted-foreground", "尚无执行结果。" }
                } else {
                    div { class: "space-y-3",
                        for result in &snapshot.results {
                            {result_detail(result)}
                        }
                    }
                }
            }
        }
    }
}

fn recent_results(snapshot: &SshDashboardSnapshot) -> Element {
    rsx! {
        div { class: "space-y-3",
            div { class: "flex flex-wrap items-center justify-between gap-3",
                div {
                    h2 { class: "text-base font-semibold", "最近结果" }
                    p { class: "text-sm text-muted-foreground", "展示最近更新的 12 条硬件和服务监测结果。" }
                }
                a { class: "text-sm font-medium text-primary hover:underline", href: page_href("results"), "查看全部" }
            }
            if snapshot.results.is_empty() {
                div { class: "rounded-md border border-dashed p-6 text-sm text-muted-foreground", "选择一个目标开始首次采集。" }
            } else {
                Table {
                    TableHeader { TableRow {
                        TableHead { "目标" }
                        TableHead { "命令" }
                        TableHead { "硬件" }
                        TableHead { "状态" }
                        TableHead { "耗时" }
                        TableHead { "时间" }
                    } }
                    TableBody {
                        for result in snapshot.results.iter().take(12) {
                            TableRow {
                                TableCell { "{result.target_name}" }
                                TableCell { "{result.command_name}" }
                                TableCell { "{hardware_label(&result.hardware_family)}" }
                                TableCell { span { class: "rounded-full px-2 py-1 text-xs font-medium {status_class(&result.status)}", "{status_label(&result.status)}" } }
                                TableCell { "{result.duration_ms} ms" }
                                TableCell { "{age_text(result.collected_at_ms)}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn result_detail(result: &SshCommandResultView) -> Element {
    rsx! {
        details { class: "rounded-md border bg-card p-4",
            summary { class: "cursor-pointer list-none",
                div { class: "flex flex-wrap items-center justify-between gap-3",
                    div {
                        div { class: "font-medium", "{result.target_name} / {result.command_name}" }
                        div { class: "mt-1 text-xs text-muted-foreground", "{result.category} · {hardware_label(&result.hardware_family)} · {result.duration_ms} ms · {age_text(result.collected_at_ms)}" }
                    }
                    span { class: "rounded-full px-2 py-1 text-xs font-medium {status_class(&result.status)}", "{status_label(&result.status)}" }
                }
            }
            div { class: "mt-4 grid gap-3 border-t pt-4",
                div { class: "text-xs text-muted-foreground", "退出码：{result.exit_code}" }
                if !result.stdout.is_empty() {
                    div {
                        div { class: "text-xs font-medium", "标准输出" }
                        pre { class: "mt-1 max-h-96 overflow-auto whitespace-pre-wrap rounded-md bg-muted p-3 text-xs", "{result.stdout}" }
                    }
                }
                if !result.stderr.is_empty() {
                    div {
                        div { class: "text-xs font-medium text-destructive", "标准错误" }
                        pre { class: "mt-1 max-h-64 overflow-auto whitespace-pre-wrap rounded-md bg-destructive/10 p-3 text-xs", "{result.stderr}" }
                    }
                }
            }
        }
    }
}

#[component]
fn FieldLabel(title: String, #[props(default)] required: bool, children: Element) -> Element {
    rsx! {
        label { class: "grid gap-2 text-sm",
            span { class: "font-medium",
                "{title}"
                if required { span { class: "text-destructive", " *" } }
            }
            {children}
        }
    }
}

fn target_status(
    target_code: &str,
    results: &[SshCommandResultView],
) -> (&'static str, &'static str) {
    let target_results = results
        .iter()
        .filter(|result| result.target_code == target_code)
        .collect::<Vec<_>>();
    if target_results.is_empty() {
        return ("未采集", "bg-slate-100 text-slate-700");
    }
    if target_results
        .iter()
        .any(|result| result.status == STATUS_FAILED)
    {
        return ("存在异常", "bg-red-100 text-red-800");
    }
    if target_results
        .iter()
        .any(|result| result.status == STATUS_SUCCESS)
    {
        return ("已采集", "bg-green-100 text-green-800");
    }
    ("硬件未匹配", "bg-amber-100 text-amber-800")
}

fn status_label(status: &str) -> &'static str {
    match status {
        STATUS_SUCCESS => "成功",
        STATUS_FAILED => "失败",
        STATUS_UNSUPPORTED => "不支持",
        _ => "未知",
    }
}

fn status_class(status: &str) -> &'static str {
    match status {
        STATUS_SUCCESS => "bg-green-100 text-green-800",
        STATUS_FAILED => "bg-red-100 text-red-800",
        STATUS_UNSUPPORTED => "bg-amber-100 text-amber-800",
        _ => "bg-slate-100 text-slate-700",
    }
}

fn auth_label(auth_type: &str) -> &'static str {
    match auth_type {
        AUTH_PRIVATE_KEY => "私钥文件",
        AUTH_PASSWORD_ENV => "密码环境变量",
        _ => "未知",
    }
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        COMMAND_KIND_MONITOR => "监测",
        "operation" => "操作",
        _ => "未知",
    }
}

fn hardware_label(family: &str) -> &str {
    match family {
        "generic" => "通用 Linux",
        "hygon_dcu" => "海光 DCU/HCU",
        "nvidia_gpu" => "NVIDIA GPU",
        "amd_rocm" => "AMD ROCm",
        "intel_xpu" => "Intel XPU",
        "ipmi" => "IPMI/BMC",
        "smart" => "SMART",
        "nvme" => "NVMe",
        "systemd" => "systemd",
        "container" => "容器运行时",
        other => other,
    }
}

fn page_href(view: &str) -> String {
    let route = format!("/ssh?view={view}");
    format!("/?route={}", urlencoding::encode(&route))
}

fn lowcode_records_href(model_name: &str) -> String {
    let route = format!("/lowcode?model={model_name}&tab=records");
    format!("/?route={}", urlencoding::encode(&route))
}

fn query_value(route: &str, key: &str) -> Option<String> {
    let query = route.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (pair_key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if pair_key != key {
            return None;
        }
        Some(
            urlencoding::decode(value)
                .map(|value| value.into_owned())
                .unwrap_or_else(|_| value.to_string()),
        )
    })
}

fn age_text(timestamp_ms: i64) -> String {
    if timestamp_ms <= 0 {
        return "未知".to_string();
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or_default();
    let seconds = now_ms.saturating_sub(timestamp_ms) / 1_000;
    match seconds {
        0..=59 => format!("{seconds} 秒前"),
        60..=3_599 => format!("{} 分钟前", seconds / 60),
        3_600..=86_399 => format!("{} 小时前", seconds / 3_600),
        _ => format!("{} 天前", seconds / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowcode_links_open_real_ssh_models() {
        assert_eq!(
            lowcode_records_href(COMMAND_MODEL),
            "/?route=%2Flowcode%3Fmodel%3Dssh_command%26tab%3Drecords"
        );
    }

    #[test]
    fn route_links_keep_server_operations_view() {
        assert_eq!(page_href("targets"), "/?route=%2Fssh%3Fview%3Dtargets");
    }
}
