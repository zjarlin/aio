use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    db::now_millis,
    error::{AppError, AppResult},
    plugin_registry::{
        CapabilityFormula, CommandContribution, ContributionBlock, EventBlock, EventDeclaration,
        MenuContribution, PlatformMatrix, PluginAiHints, PluginEntry, PluginFormula, PluginParent,
        ViewContribution,
    },
    plugin_store::{
        InstalledPluginRecord, PluginPackageSignaturePlaceholder, PluginRegistryStore,
        RegistryAuditRecord, RegistryLockRecord,
    },
};

const DEFAULT_PARENT_PLUGIN_ID: &str = "asset-suite";
const DEFAULT_PARENT_MOUNT: &str = "asset-suite.assetView";
const PLUGIN_DRAFT_SCHEMA_VERSION: &str = "plugin-draft/v1";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCreateFromPromptInput {
    pub prompt: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub parent_plugin_id: Option<String>,
    #[serde(default)]
    pub parent_mount: Option<String>,
    #[serde(default)]
    pub route_path: Option<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPermissionPlanItem {
    pub id: String,
    pub scope: String,
    pub reason: String,
    pub allow: Vec<String>,
    pub optional: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDraft {
    pub schema_version: String,
    pub prompt: String,
    pub formula: PluginFormula,
    pub permission_plan: Vec<PluginPermissionPlanItem>,
    pub generated_files: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDraftWriteResult {
    pub output_dir: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDraftCreationResult {
    pub draft: PluginDraft,
    #[serde(default)]
    pub write_result: Option<PluginDraftWriteResult>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDiagnosticsPackage {
    pub plugin_id: String,
    pub run_id: String,
    pub status: String,
    #[serde(default)]
    pub formula_errors: Vec<String>,
    #[serde(default)]
    pub permission_errors: Vec<PluginDiagnosticPermissionError>,
    #[serde(default)]
    pub platform_errors: Vec<PluginDiagnosticPlatformError>,
    #[serde(default)]
    pub test_failures: Vec<String>,
    #[serde(default)]
    pub ui_preview: Option<PluginDiagnosticUiPreview>,
    #[serde(default)]
    pub repair_hint: String,
    #[serde(default)]
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDiagnosticPermissionError {
    pub capability: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDiagnosticPlatformError {
    pub platform: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDiagnosticUiPreview {
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub dom_snapshots: Vec<PluginDiagnosticDomSnapshot>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDiagnosticDomSnapshot {
    pub id: String,
    pub html: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRepairFromDiagnosticsInput {
    pub diagnostics_path: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginVerifyDraftInput {
    pub source_path: String,
    #[serde(default)]
    pub output_dir: Option<String>,
    #[serde(default)]
    pub write: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginVerificationReport {
    pub schema_version: String,
    pub plugin_id: String,
    pub run_id: String,
    pub status: String,
    pub checks: Vec<PluginVerificationCheck>,
    pub diagnostics: PluginDiagnosticsPackage,
    #[serde(default)]
    pub written_files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginVerificationCheck {
    pub id: String,
    pub title: String,
    pub status: String,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPublishGateReport {
    pub schema_version: String,
    pub plugin_id: String,
    pub run_id: String,
    pub status: String,
    pub checks: Vec<PluginVerificationCheck>,
    pub content_hash: String,
    pub verification: PluginVerificationReport,
    pub lock: RegistryLockRecord,
    pub signature: PluginPackageSignaturePlaceholder,
    pub audit: RegistryAuditRecord,
    pub remote_registry_protocol_path: String,
    #[serde(default)]
    pub written_files: Vec<String>,
}

pub fn create_from_prompt(input: PluginCreateFromPromptInput) -> AppResult<PluginDraft> {
    let prompt = normalize_prompt(&input.prompt)?;
    let display_name = input
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| infer_display_name(prompt));
    let id = input
        .id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(validate_plugin_id)
        .transpose()?
        .unwrap_or_else(|| infer_plugin_id(prompt, &display_name, input.kind.as_deref()));
    let kind = normalize_kind(input.kind.as_deref(), input.parent_plugin_id.as_deref())?;
    let route_path = input
        .route_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(normalize_route_path)
        .transpose()?
        .unwrap_or_else(|| infer_route_path(&id, &kind, prompt));
    let parent = build_parent(
        &kind,
        input.parent_plugin_id.as_deref(),
        input.parent_mount.as_deref(),
    );
    let capabilities = infer_capabilities(prompt);
    let command_id = format!("{id}.run");
    let command_capabilities = capabilities
        .iter()
        .map(|capability| capability.id.clone())
        .collect::<Vec<_>>();
    let menu = menu_for(&id, &display_name, &route_path);
    let view = ViewContribution {
        id: format!("{id}.main"),
        slot: "main".to_string(),
        schema: infer_view_schema(prompt),
        contract: "schemas/plugin-ui-view.v1.schema.json".to_string(),
        path: route_path.clone(),
        asset_item_kind: String::new(),
        when: String::new(),
    };

    let formula = PluginFormula {
        schema_version: "plugin-formula/v1".to_string(),
        id: id.clone(),
        kind,
        display_name: display_name.clone(),
        version: "0.1.0".to_string(),
        trust_level: "community".to_string(),
        intent: prompt.to_string(),
        entry: Some(PluginEntry {
            node: "./src/index.ts".to_string(),
            browser: String::new(),
            worker: String::new(),
            remote: String::new(),
        }),
        parent,
        activation: vec![format!("onCommand:{command_id}"), format!("onRoute:{route_path}")],
        platforms: Some(platform_matrix_for(&capabilities)),
        capabilities: capabilities.clone(),
        contributes: ContributionBlock {
            commands: vec![CommandContribution {
                id: command_id,
                title: format!("Run {display_name}"),
                category: category_for(&id),
                capabilities: command_capabilities,
                when: String::new(),
            }],
            menus: vec![menu],
            views: vec![view],
            ..ContributionBlock::default()
        },
        events: EventBlock {
            publishes: vec![EventDeclaration {
                event: format!("{id}.activated"),
                schema: "schemas/events/platform-event-envelope.v1.schema.json".to_string(),
            }],
            subscribes: Vec::new(),
        },
        expectations: vec![
            "formula.json remains the source of truth for metadata and permissions.".to_string(),
            "All native or sensitive operations must go through Capability Broker.".to_string(),
            "Keep the capsule compact until a file exceeds 600 lines or mixes unrelated responsibilities.".to_string(),
        ],
        ai: PluginAiHints {
            editable_by_ai: true,
            primary_edit_file: "formula.json".to_string(),
            preferred_files: vec![
                "formula.json".to_string(),
                "src/index.ts".to_string(),
                "tests/smoke.test.ts".to_string(),
            ],
            do_not_split_below: "Keep the generated plugin capsule small and formula-first.".to_string(),
            role: "generated-plugin".to_string(),
            primary_output: "formula.json".to_string(),
            repair_strategy: "formula-first".to_string(),
        },
        child_policy: None,
    };

    let permission_plan = permission_plan_from_capabilities(&capabilities);

    Ok(PluginDraft {
        schema_version: PLUGIN_DRAFT_SCHEMA_VERSION.to_string(),
        prompt: prompt.to_string(),
        formula,
        permission_plan,
        generated_files: generated_files(),
        diagnostics: Vec::new(),
    })
}

pub fn create_from_prompt_and_write(
    input: PluginCreateFromPromptInput,
) -> AppResult<PluginDraftCreationResult> {
    let write_input = input.output_dir.clone();
    let force = input.force;
    let draft = create_from_prompt(input)?;
    let write_result = if let Some(output_dir) = write_input {
        Some(write_plugin_draft(&draft, output_dir, force)?)
    } else {
        None
    };
    Ok(PluginDraftCreationResult {
        draft,
        write_result,
    })
}

pub fn write_plugin_draft(
    draft: &PluginDraft,
    output_dir: impl AsRef<Path>,
    force: bool,
) -> AppResult<PluginDraftWriteResult> {
    let output_dir = output_dir.as_ref();
    ensure_output_dir(output_dir, force)?;

    let files = [
        (
            "formula.json",
            serde_json::to_string_pretty(&draft.formula)?,
        ),
        ("permission-plan.md", permission_plan_markdown(draft)),
        ("context-pack.md", context_pack_markdown(draft)),
        ("PLAN.md", plan_markdown()),
        ("README.md", readme_markdown(draft)),
        ("src/index.ts", source_stub(draft)),
        ("tests/smoke.test.ts", smoke_test_stub(draft)),
    ];

    let mut written = Vec::with_capacity(files.len());
    for (relative, content) in files {
        let path = output_dir.join(relative);
        write_managed_file(&path, &content, force)?;
        written.push(path.to_string_lossy().into_owned());
    }

    Ok(PluginDraftWriteResult {
        output_dir: output_dir.to_string_lossy().into_owned(),
        files: written,
    })
}

pub fn publish_local(
    source_path: impl AsRef<Path>,
    data_dir: impl AsRef<Path>,
) -> AppResult<InstalledPluginRecord> {
    let source_path = source_path.as_ref();
    publish_gate(source_path, data_dir.as_ref(), true)?;
    PluginRegistryStore::new(data_dir).install(source_path)
}

pub fn publish_gate(
    source_path: impl AsRef<Path>,
    data_dir: impl AsRef<Path>,
    write: bool,
) -> AppResult<PluginPublishGateReport> {
    let source_path = source_path.as_ref();
    let data_dir = data_dir.as_ref();
    if !source_path.is_dir() {
        return Err(AppError::BadRequest(format!(
            "plugin publish gate sourcePath is not a directory: {}",
            source_path.display()
        )));
    }

    let store = PluginRegistryStore::new(data_dir);
    store.ensure_layout()?;
    let verification = verify_plugin_draft(PluginVerifyDraftInput {
        source_path: source_path.to_string_lossy().into_owned(),
        output_dir: None,
        write,
    })?;
    let formula: PluginFormula = read_json_file(&source_path.join("formula.json"))?;
    let content_hash = hash_directory(source_path)?;
    let now = now_millis();
    let run_id = format!("publish-gate-{}", current_unix_millis());
    let package_lock_path = source_path.join("lock.json");
    let signature_path = source_path.join("signature.json");
    let package_audit_path = source_path.join("audit.jsonl");
    let remote_protocol_path = store.remote_protocol_path();

    let lock = RegistryLockRecord {
        content_hash: content_hash.clone(),
        enabled: true,
        id: formula.id.clone(),
        locked_at: now,
        registry_path: store
            .plugins_dir()
            .join(&formula.id)
            .to_string_lossy()
            .into_owned(),
        schema_version: formula.schema_version.clone(),
        signature_path: signature_path.to_string_lossy().into_owned(),
        source_path: source_path.to_string_lossy().into_owned(),
        updated_at: now,
        version: formula.version.clone(),
    };
    let signature = PluginPackageSignaturePlaceholder {
        algorithm: "sha256-placeholder".to_string(),
        content_hash: content_hash.clone(),
        created_at: now,
        key_id: "local-dev-placeholder".to_string(),
        plugin_id: formula.id.clone(),
        reason:
            "Publish gate records a placeholder signature until real signing keys are configured."
                .to_string(),
        schema_version: "plugin-signature-placeholder/v1".to_string(),
        signature: format!("unsigned:{content_hash}"),
        status: "placeholder".to_string(),
    };
    let audit = RegistryAuditRecord {
        action: "publish-gate".to_string(),
        content_hash: Some(content_hash.clone()),
        detail: Some(format!(
            "verification={} lock={} signature={} remoteProtocol={}",
            verification.status,
            package_lock_path.display(),
            signature_path.display(),
            remote_protocol_path.display()
        )),
        id: formula.id.clone(),
        path: Some(source_path.to_string_lossy().into_owned()),
        status: "pending".to_string(),
        timestamp: now,
    };

    let mut checks = Vec::new();
    if verification.status == "passed" {
        checks.push(passed_check(
            "verification.passed",
            "Verification report passed",
            format!("{} passed", verification.run_id),
        ));
    } else {
        checks.push(failed_check(
            "verification.passed",
            "Verification report passed",
            format!("verification status is {}", verification.status),
        ));
    }
    push_file_check(
        &mut checks,
        "lock.present",
        "Package lock present",
        &package_lock_path,
        write,
    );
    push_file_check(
        &mut checks,
        "signature.placeholder",
        "Signature placeholder present",
        &signature_path,
        write,
    );
    push_file_check(
        &mut checks,
        "audit.intent",
        "Package audit intent present",
        &package_audit_path,
        write,
    );
    if remote_protocol_path.is_file() {
        checks.push(passed_check(
            "remote.protocol",
            "Remote registry protocol draft",
            remote_protocol_path.to_string_lossy().into_owned(),
        ));
    } else {
        checks.push(failed_check(
            "remote.protocol",
            "Remote registry protocol draft",
            "remote-registry.protocol.json is missing".to_string(),
        ));
    }
    if formula.kind == "child-plugin" {
        if let Some(parent) = formula.parent.as_ref() {
            if parent.compatible_parent_range.trim().is_empty() {
                checks.push(failed_check(
                    "parent.compatibility",
                    "Parent compatibility",
                    "compatibleParentRange is missing".to_string(),
                ));
            } else {
                checks.push(passed_check(
                    "parent.compatibility",
                    "Parent compatibility",
                    format!(
                        "{} mounts {} {}",
                        formula.id, parent.plugin_id, parent.compatible_parent_range
                    ),
                ));
            }
        } else {
            checks.push(failed_check(
                "parent.compatibility",
                "Parent compatibility",
                "child plugin is missing parent".to_string(),
            ));
        }
    }

    let status = if checks.iter().any(|check| check.status == "failed") {
        "failed"
    } else {
        "passed"
    };
    let mut written_files = verification.written_files.clone();

    if write {
        fs::write(&package_lock_path, serde_json::to_string_pretty(&lock)?)?;
        fs::write(&signature_path, serde_json::to_string_pretty(&signature)?)?;
        append_jsonl(&package_audit_path, &audit)?;
        written_files.push(package_lock_path.to_string_lossy().into_owned());
        written_files.push(signature_path.to_string_lossy().into_owned());
        written_files.push(package_audit_path.to_string_lossy().into_owned());
    }

    if status != "passed" {
        return Err(AppError::BadRequest(format!(
            "publish gate failed for {}: {}",
            formula.id,
            checks
                .iter()
                .filter(|check| check.status == "failed")
                .map(|check| check.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    Ok(PluginPublishGateReport {
        schema_version: "plugin-publish-gate/v1".to_string(),
        plugin_id: formula.id,
        run_id,
        status: status.to_string(),
        checks,
        content_hash,
        verification,
        lock,
        signature,
        audit,
        remote_registry_protocol_path: remote_protocol_path.to_string_lossy().into_owned(),
        written_files,
    })
}

pub fn repair_from_diagnostics(
    input: PluginRepairFromDiagnosticsInput,
) -> AppResult<PluginDraftCreationResult> {
    let diagnostics: PluginDiagnosticsPackage = read_json_file(Path::new(&input.diagnostics_path))?;
    let source_path = input
        .source_path
        .or(diagnostics.source_path.clone())
        .ok_or_else(|| {
            AppError::BadRequest("diagnostics package is missing sourcePath".to_string())
        })?;
    let source_path = PathBuf::from(source_path);
    let formula_path = source_path.join("formula.json");
    let mut formula: PluginFormula = read_json_file(&formula_path)?;
    apply_diagnostics_repairs(&mut formula, &diagnostics);

    let draft = PluginDraft {
        schema_version: PLUGIN_DRAFT_SCHEMA_VERSION.to_string(),
        prompt: formula.intent.clone(),
        permission_plan: permission_plan_from_capabilities(&formula.capabilities),
        generated_files: generated_files(),
        diagnostics: diagnostics_summary(&diagnostics),
        formula,
    };
    let write_result = Some(write_plugin_draft(&draft, &source_path, input.force)?);

    Ok(PluginDraftCreationResult {
        draft,
        write_result,
    })
}

pub fn verify_plugin_draft(input: PluginVerifyDraftInput) -> AppResult<PluginVerificationReport> {
    let source_path = PathBuf::from(&input.source_path);
    if !source_path.is_dir() {
        return Err(AppError::BadRequest(format!(
            "plugin verify sourcePath is not a directory: {}",
            source_path.display()
        )));
    }

    let formula_path = source_path.join("formula.json");
    let run_id = format!("verify-{}", current_unix_millis());
    let mut checks = Vec::new();
    let mut formula_errors = Vec::new();
    let mut permission_errors = Vec::new();
    let mut platform_errors = Vec::new();
    let mut test_failures = Vec::new();

    let formula_result = read_json_file::<PluginFormula>(&formula_path);
    match &formula_result {
        Ok(formula) => {
            checks.push(passed_check(
                "formula.parse",
                "Parse formula.json",
                format!("{} parsed", formula.id),
            ));
        }
        Err(error) => {
            let message = error.to_string();
            formula_errors.push(message.clone());
            checks.push(failed_check("formula.parse", "Parse formula.json", message));
        }
    }

    let fallback_formula = PluginFormula {
        schema_version: "plugin-formula/v1".to_string(),
        id: "unknown".to_string(),
        kind: "plugin".to_string(),
        display_name: "Unknown".to_string(),
        version: String::new(),
        trust_level: "community".to_string(),
        intent: String::new(),
        entry: None,
        parent: None,
        activation: Vec::new(),
        platforms: None,
        capabilities: Vec::new(),
        contributes: ContributionBlock::default(),
        events: EventBlock::default(),
        expectations: Vec::new(),
        ai: PluginAiHints::default(),
        child_policy: None,
    };
    let formula = formula_result.as_ref().unwrap_or(&fallback_formula);

    validate_permission_plan(&source_path, formula, &mut checks, &mut permission_errors);
    validate_platform_matrix(formula, &mut checks, &mut platform_errors);
    validate_smoke_test(&source_path, formula, &mut checks, &mut test_failures);
    validate_context_pack(&source_path, &mut checks);

    let status = if !formula_errors.is_empty()
        || !permission_errors.is_empty()
        || !platform_errors.is_empty()
        || !test_failures.is_empty()
    {
        "failed"
    } else {
        "passed"
    };

    let ui_preview = build_ui_preview(formula);
    checks.push(passed_check(
        "ui.preview",
        "Generate UI preview DOM snapshot",
        format!("{} DOM snapshot(s)", ui_preview.dom_snapshots.len()),
    ));

    let diagnostics = PluginDiagnosticsPackage {
        plugin_id: formula.id.clone(),
        run_id: run_id.clone(),
        status: status.to_string(),
        formula_errors,
        permission_errors,
        platform_errors,
        test_failures,
        ui_preview: Some(ui_preview),
        repair_hint: repair_hint_for(&checks),
        source_path: Some(source_path.to_string_lossy().into_owned()),
    };

    let mut report = PluginVerificationReport {
        schema_version: "plugin-verification/v1".to_string(),
        plugin_id: formula.id.clone(),
        run_id,
        status: status.to_string(),
        checks,
        diagnostics,
        written_files: Vec::new(),
    };

    if input.write {
        let output_dir = input
            .output_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| source_path.clone());
        fs::create_dir_all(&output_dir)?;
        if let Some(ui_preview) = report.diagnostics.ui_preview.as_mut() {
            for snapshot in &mut ui_preview.dom_snapshots {
                let file_name = format!("ui-preview.{}.dom.html", safe_file_segment(&snapshot.id));
                let preview_path = output_dir.join(file_name);
                fs::write(&preview_path, &snapshot.html)?;
                snapshot.path = Some(preview_path.to_string_lossy().into_owned());
                report
                    .written_files
                    .push(preview_path.to_string_lossy().into_owned());
            }
        }
        let diagnostics_path = output_dir.join("diagnostics.json");
        let verification_path = output_dir.join("verification.json");
        report
            .written_files
            .push(diagnostics_path.to_string_lossy().into_owned());
        report
            .written_files
            .push(verification_path.to_string_lossy().into_owned());
        fs::write(
            &diagnostics_path,
            serde_json::to_string_pretty(&report.diagnostics)?,
        )?;
        fs::write(&verification_path, serde_json::to_string_pretty(&report)?)?;
    }

    Ok(report)
}

fn build_ui_preview(formula: &PluginFormula) -> PluginDiagnosticUiPreview {
    let mut html = String::new();
    html.push_str("<section data-aio-plugin-preview=\"true\"");
    html.push_str(&format!(
        " data-plugin-id=\"{}\" data-plugin-kind=\"{}\">",
        escape_html_attr(&formula.id),
        escape_html_attr(&formula.kind),
    ));
    html.push_str(&format!(
        "<header><h1>{}</h1><p>{}</p></header>",
        escape_html_text(&formula.display_name),
        escape_html_text(&formula.intent),
    ));

    if formula.contributes.views.is_empty() {
        html.push_str(
            "<section data-empty=\"views\"><p>No declarative views contributed.</p></section>",
        );
    } else {
        html.push_str("<main data-preview-region=\"views\">");
        for view in &formula.contributes.views {
            html.push_str(&format!(
                "<article data-view-id=\"{}\" data-view-schema=\"{}\" data-view-slot=\"{}\" data-view-path=\"{}\" data-view-when=\"{}\">",
                escape_html_attr(&view.id),
                escape_html_attr(&view.schema),
                escape_html_attr(&view.slot),
                escape_html_attr(&view.path),
                escape_html_attr(&view.when),
            ));
            html.push_str(&format!(
                "<h2>{}</h2><dl><dt>schema</dt><dd>{}</dd><dt>slot</dt><dd>{}</dd><dt>path</dt><dd>{}</dd><dt>contract</dt><dd>{}</dd></dl>",
                escape_html_text(&view.id),
                escape_html_text(&view.schema),
                escape_html_text(&view.slot),
                escape_html_text(&view.path),
                escape_html_text(&view.contract),
            ));
            html.push_str("</article>");
        }
        html.push_str("</main>");
    }

    if !formula.contributes.commands.is_empty() {
        html.push_str("<nav data-preview-region=\"commands\"><ul>");
        for command in &formula.contributes.commands {
            html.push_str(&format!(
                "<li data-command-id=\"{}\" data-command-when=\"{}\">{}</li>",
                escape_html_attr(&command.id),
                escape_html_attr(&command.when),
                escape_html_text(&command.title),
            ));
        }
        html.push_str("</ul></nav>");
    }

    html.push_str("</section>");
    PluginDiagnosticUiPreview {
        screenshots: Vec::new(),
        dom_snapshots: vec![PluginDiagnosticDomSnapshot {
            id: "formula-view-preview".to_string(),
            html,
            path: None,
        }],
    }
}

fn safe_file_segment(value: &str) -> String {
    let safe = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    safe.trim_matches('-').to_string()
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_attr(value: &str) -> String {
    escape_html_text(value).replace('"', "&quot;")
}

fn normalize_prompt(prompt: &str) -> AppResult<&str> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err(AppError::BadRequest(
            "plugin.createFromPrompt requires a non-empty prompt".to_string(),
        ));
    }
    Ok(prompt)
}

fn normalize_kind(kind: Option<&str>, parent_plugin_id: Option<&str>) -> AppResult<String> {
    let kind = kind
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            if parent_plugin_id.is_some() {
                "child-plugin"
            } else {
                "plugin"
            }
        });
    match kind {
        "plugin" | "child-plugin" => Ok(kind.to_string()),
        other => Err(AppError::BadRequest(format!(
            "unsupported plugin kind for createFromPrompt: {other}"
        ))),
    }
}

fn validate_plugin_id(id: &str) -> AppResult<String> {
    let valid = id.chars().enumerate().all(|(index, character)| {
        let allowed = character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-');
        allowed && (index > 0 || character.is_ascii_alphanumeric())
    });
    if valid {
        Ok(id.to_string())
    } else {
        Err(AppError::BadRequest(format!(
            "invalid plugin id for createFromPrompt: {id}"
        )))
    }
}

fn normalize_route_path(route_path: &str) -> AppResult<String> {
    if route_path.starts_with('/') {
        Ok(route_path.to_string())
    } else {
        Err(AppError::BadRequest(format!(
            "routePath must start with '/': {route_path}"
        )))
    }
}

fn build_parent(
    kind: &str,
    parent_plugin_id: Option<&str>,
    parent_mount: Option<&str>,
) -> Option<PluginParent> {
    if kind != "child-plugin" {
        return None;
    }
    Some(PluginParent {
        plugin_id: parent_plugin_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_PARENT_PLUGIN_ID)
            .to_string(),
        mount: parent_mount
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_PARENT_MOUNT)
            .to_string(),
        compatible_parent_range: "^1.0.0".to_string(),
    })
}

fn infer_display_name(prompt: &str) -> String {
    let first_line = prompt.lines().next().unwrap_or(prompt).trim();
    let mut display_name = first_line.chars().take(48).collect::<String>();
    if display_name.is_empty() {
        display_name = "Generated Plugin".to_string();
    }
    display_name
}

fn infer_plugin_id(prompt: &str, display_name: &str, kind: Option<&str>) -> String {
    let slug = slug_segment(display_name)
        .or_else(|| slug_segment(prompt))
        .unwrap_or_else(|| format!("generated-{}", short_hash(prompt)));
    if kind == Some("child-plugin") || contains_any(prompt, &["资产", "asset", "assets"]) {
        format!("asset.{slug}")
    } else {
        format!("workspace.{slug}")
    }
}

fn infer_route_path(id: &str, kind: &str, prompt: &str) -> String {
    let tail = id.rsplit('.').next().unwrap_or(id).replace('_', "-");
    if kind == "child-plugin" || contains_any(prompt, &["资产", "asset", "assets"]) {
        format!("/assets/{tail}")
    } else {
        format!("/plugins/{tail}")
    }
}

fn infer_capabilities(prompt: &str) -> Vec<CapabilityFormula> {
    let mut capabilities = Vec::new();
    if contains_any(
        prompt,
        &[
            "数据库",
            "记录",
            "列表",
            "新增",
            "编辑",
            "删除",
            "管理",
            "db",
            "sqlite",
            "crud",
            "table",
        ],
    ) {
        capabilities.push(capability(
            "db.sqlite.read",
            "app-data",
            "Read records required by the generated plugin.",
        ));
        capabilities.push(capability(
            "db.sqlite.write",
            "app-data",
            "Create or update records required by the generated plugin.",
        ));
    }
    if contains_any(
        prompt,
        &[
            "文件",
            "目录",
            "读取",
            "扫描",
            "导入",
            "file",
            "directory",
            "scan",
            "import",
        ],
    ) {
        capabilities.push(capability(
            "fs.read",
            "user-selected-directory",
            "Read user-approved files or directories.",
        ));
    }
    if contains_any(
        prompt,
        &[
            "写入", "保存", "生成", "部署", "导出", "write", "save", "deploy", "export",
        ],
    ) {
        capabilities.push(capability(
            "fs.write",
            "user-selected-directory",
            "Write only to paths approved by the user.",
        ));
    }
    if contains_any(
        prompt,
        &["git", "命令", "执行", "shell", "process", "exec", "cli"],
    ) {
        let mut process = capability(
            "process.exec",
            "workspace",
            "Execute allow-listed local commands through Capability Broker.",
        );
        if contains_any(prompt, &["git"]) {
            process.allow.push("git".to_string());
        }
        capabilities.push(process);
    }
    if contains_any(
        prompt,
        &[
            "api", "http", "https", "网络", "请求", "openai", "github", "fetch", "network",
        ],
    ) {
        let scope = if contains_any(prompt, &["openai"]) {
            "https://api.openai.com"
        } else if contains_any(prompt, &["github"]) {
            "https://api.github.com"
        } else {
            "user-approved-endpoints"
        };
        capabilities.push(capability(
            "network.fetch",
            scope,
            "Call explicitly declared remote APIs.",
        ));
    }

    dedupe_capabilities(capabilities)
}

fn permission_plan_from_capabilities(
    capabilities: &[CapabilityFormula],
) -> Vec<PluginPermissionPlanItem> {
    capabilities
        .iter()
        .map(|capability| PluginPermissionPlanItem {
            id: capability.id.clone(),
            scope: capability.scope.clone(),
            reason: capability.reason.clone(),
            allow: capability.allow.clone(),
            optional: capability.optional,
        })
        .collect()
}

fn apply_diagnostics_repairs(formula: &mut PluginFormula, diagnostics: &PluginDiagnosticsPackage) {
    let hint = diagnostics.repair_hint.to_lowercase();
    if hint.contains("git")
        || diagnostics
            .permission_errors
            .iter()
            .any(|issue| issue.reason.to_lowercase().contains("git"))
    {
        if let Some(process) = formula
            .capabilities
            .iter_mut()
            .find(|capability| capability.id == "process.exec")
        {
            if !process.allow.iter().any(|allow| allow == "git") {
                process.allow.push("git".to_string());
            }
        }
    }

    if diagnostics
        .platform_errors
        .iter()
        .any(|issue| issue.platform.eq_ignore_ascii_case("web"))
    {
        let platforms = formula.platforms.get_or_insert_with(|| PlatformMatrix {
            supported: vec![
                "macos".to_string(),
                "windows".to_string(),
                "linux".to_string(),
                "web".to_string(),
                "remote".to_string(),
            ],
            degraded: Vec::new(),
            unsupported: Vec::new(),
            reason: "Repaired from diagnostics".to_string(),
        });
        if !platforms.degraded.iter().any(|platform| platform == "web") {
            platforms.degraded.push("web".to_string());
        }
        let fallback =
            "Web shell must degrade to a read-only view when native execution is unavailable.";
        if !formula
            .expectations
            .iter()
            .any(|expectation| expectation == fallback)
        {
            formula.expectations.push(fallback.to_string());
        }
    }

    for issue in &diagnostics.permission_errors {
        if !formula
            .capabilities
            .iter()
            .any(|capability| capability.id == issue.capability)
        {
            formula
                .capabilities
                .push(capability_for_issue(&issue.capability, &issue.reason));
        }
    }
    formula.capabilities = dedupe_capabilities(formula.capabilities.clone());
}

fn capability_for_issue(id: &str, reason: &str) -> CapabilityFormula {
    let scope = match id {
        "db.sqlite.read" | "db.sqlite.write" => "app-data",
        "fs.read" | "fs.write" => "user-selected-directory",
        "process.exec" => "workspace",
        "network.fetch" => "user-approved-endpoints",
        _ => "app-data",
    };
    let mut capability = capability(id, scope, reason);
    if id == "process.exec" && reason.to_lowercase().contains("git") {
        capability.allow.push("git".to_string());
    }
    capability
}

fn diagnostics_summary(diagnostics: &PluginDiagnosticsPackage) -> Vec<String> {
    let mut summary = Vec::new();
    if !diagnostics.formula_errors.is_empty() {
        summary.push(format!(
            "formulaErrors: {}",
            diagnostics.formula_errors.join(" | ")
        ));
    }
    if !diagnostics.permission_errors.is_empty() {
        summary.push(format!(
            "permissionErrors: {}",
            diagnostics
                .permission_errors
                .iter()
                .map(|issue| format!("{}: {}", issue.capability, issue.reason))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    if !diagnostics.platform_errors.is_empty() {
        summary.push(format!(
            "platformErrors: {}",
            diagnostics
                .platform_errors
                .iter()
                .map(|issue| format!("{}: {}", issue.platform, issue.reason))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    if !diagnostics.test_failures.is_empty() {
        summary.push(format!(
            "testFailures: {}",
            diagnostics.test_failures.join(" | ")
        ));
    }
    if summary.is_empty() && !diagnostics.repair_hint.trim().is_empty() {
        summary.push(diagnostics.repair_hint.clone());
    }
    summary
}

fn validate_permission_plan(
    source_path: &Path,
    formula: &PluginFormula,
    checks: &mut Vec<PluginVerificationCheck>,
    permission_errors: &mut Vec<PluginDiagnosticPermissionError>,
) {
    let permission_plan_path = source_path.join("permission-plan.md");
    let permission_plan = fs::read_to_string(&permission_plan_path).unwrap_or_default();
    if formula.capabilities.is_empty() {
        checks.push(passed_check(
            "permission.plan",
            "Check permission plan",
            "No sensitive capabilities requested".to_string(),
        ));
        return;
    }
    if permission_plan.trim().is_empty() {
        for capability in &formula.capabilities {
            permission_errors.push(PluginDiagnosticPermissionError {
                capability: capability.id.clone(),
                reason: "permission-plan.md is missing or empty".to_string(),
            });
        }
        checks.push(failed_check(
            "permission.plan",
            "Check permission plan",
            "permission-plan.md is missing or empty".to_string(),
        ));
        return;
    }

    let missing = formula
        .capabilities
        .iter()
        .filter(|capability| !permission_plan.contains(&format!("`{}`", capability.id)))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        checks.push(passed_check(
            "permission.plan",
            "Check permission plan",
            "permission-plan.md covers all declared capabilities".to_string(),
        ));
    } else {
        for capability in missing {
            permission_errors.push(PluginDiagnosticPermissionError {
                capability: capability.id.clone(),
                reason: "permission-plan.md does not mention declared capability".to_string(),
            });
        }
        checks.push(failed_check(
            "permission.plan",
            "Check permission plan",
            "permission-plan.md is missing declared capabilities".to_string(),
        ));
    }
}

fn validate_platform_matrix(
    formula: &PluginFormula,
    checks: &mut Vec<PluginVerificationCheck>,
    platform_errors: &mut Vec<PluginDiagnosticPlatformError>,
) {
    let required = ["macos", "windows", "linux", "web", "remote"];
    let Some(platforms) = &formula.platforms else {
        for platform in required {
            platform_errors.push(PluginDiagnosticPlatformError {
                platform: platform.to_string(),
                reason: "platform matrix is missing".to_string(),
            });
        }
        checks.push(failed_check(
            "platform.matrix",
            "Check platform matrix",
            "platforms must explicitly cover macos/windows/linux/web/remote".to_string(),
        ));
        return;
    };

    let missing = required
        .iter()
        .filter(|platform| {
            !platforms.supported.iter().any(|value| value == **platform)
                && !platforms.degraded.iter().any(|value| value == **platform)
                && !platforms
                    .unsupported
                    .iter()
                    .any(|value| value == **platform)
        })
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        checks.push(passed_check(
            "platform.matrix",
            "Check platform matrix",
            "platform matrix covers macos/windows/linux/web/remote".to_string(),
        ));
    } else {
        for platform in missing {
            platform_errors.push(PluginDiagnosticPlatformError {
                platform: platform.to_string(),
                reason: "platform is not covered by supported/degraded/unsupported".to_string(),
            });
        }
        checks.push(failed_check(
            "platform.matrix",
            "Check platform matrix",
            "platform matrix has missing platforms".to_string(),
        ));
    }
}

fn validate_smoke_test(
    source_path: &Path,
    formula: &PluginFormula,
    checks: &mut Vec<PluginVerificationCheck>,
    test_failures: &mut Vec<String>,
) {
    let smoke_test_path = source_path.join("tests/smoke.test.ts");
    let smoke_test = fs::read_to_string(&smoke_test_path).unwrap_or_default();
    if smoke_test.trim().is_empty() {
        test_failures.push("tests/smoke.test.ts is missing or empty".to_string());
        checks.push(failed_check(
            "smoke.test",
            "Check smoke test",
            "tests/smoke.test.ts is missing or empty".to_string(),
        ));
        return;
    }
    if !smoke_test.contains(&formula.id) || !smoke_test.contains("plugin-formula/v1") {
        test_failures.push(
            "tests/smoke.test.ts does not assert formula identity and schema version".to_string(),
        );
        checks.push(failed_check(
            "smoke.test",
            "Check smoke test",
            "smoke test must assert plugin id and plugin-formula/v1".to_string(),
        ));
        return;
    }
    checks.push(passed_check(
        "smoke.test",
        "Check smoke test",
        "smoke test asserts formula identity".to_string(),
    ));
}

fn validate_context_pack(source_path: &Path, checks: &mut Vec<PluginVerificationCheck>) {
    let context_pack_path = source_path.join("context-pack.md");
    let context_pack = fs::read_to_string(&context_pack_path).unwrap_or_default();
    if context_pack.contains("## Intent")
        && context_pack.contains("## Permission Plan")
        && context_pack.contains("## File Map")
    {
        checks.push(passed_check(
            "context-pack",
            "Check Agent context pack",
            "context-pack.md contains intent, permission plan and file map".to_string(),
        ));
    } else {
        checks.push(warning_check(
            "context-pack",
            "Check Agent context pack",
            "context-pack.md is missing recommended sections".to_string(),
        ));
    }
}

fn repair_hint_for(checks: &[PluginVerificationCheck]) -> String {
    let failed = checks
        .iter()
        .filter(|check| check.status == "failed")
        .map(|check| check.id.as_str())
        .collect::<Vec<_>>();
    if failed.is_empty() {
        return "Verification passed; draft can proceed to publish gate.".to_string();
    }
    if failed.contains(&"formula.parse") {
        return "Repair formula.json first; it must parse as plugin-formula/v1.".to_string();
    }
    if failed.contains(&"permission.plan") {
        return "Regenerate permission-plan.md from formula capabilities.".to_string();
    }
    if failed.contains(&"platform.matrix") {
        return "Add explicit macos/windows/linux/web/remote platform matrix entries.".to_string();
    }
    if failed.contains(&"smoke.test") {
        return "Regenerate tests/smoke.test.ts so it asserts formula identity.".to_string();
    }
    "Repair failed verification checks before publishing.".to_string()
}

fn passed_check(id: &str, title: &str, detail: String) -> PluginVerificationCheck {
    PluginVerificationCheck {
        id: id.to_string(),
        title: title.to_string(),
        status: "passed".to_string(),
        detail,
    }
}

fn failed_check(id: &str, title: &str, detail: String) -> PluginVerificationCheck {
    PluginVerificationCheck {
        id: id.to_string(),
        title: title.to_string(),
        status: "failed".to_string(),
        detail,
    }
}

fn warning_check(id: &str, title: &str, detail: String) -> PluginVerificationCheck {
    PluginVerificationCheck {
        id: id.to_string(),
        title: title.to_string(),
        status: "warning".to_string(),
        detail,
    }
}

fn push_file_check(
    checks: &mut Vec<PluginVerificationCheck>,
    id: &str,
    title: &str,
    path: &Path,
    write: bool,
) {
    if path.is_file() || write {
        checks.push(passed_check(id, title, path.to_string_lossy().into_owned()));
    } else {
        checks.push(failed_check(
            id,
            title,
            format!("missing {}", path.display()),
        ));
    }
}

fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn current_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn read_json_file<T>(path: &Path) -> AppResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    let value = fs::read_to_string(path)?;
    serde_json::from_str(&value)
        .map_err(|source| AppError::BadRequest(format!("解析 {} 失败：{source}", path.display())))
}

fn capability(id: &str, scope: &str, reason: &str) -> CapabilityFormula {
    CapabilityFormula {
        id: id.to_string(),
        scope: scope.to_string(),
        allow: Vec::new(),
        reason: reason.to_string(),
        platforms: Vec::new(),
        optional: false,
    }
}

fn dedupe_capabilities(capabilities: Vec<CapabilityFormula>) -> Vec<CapabilityFormula> {
    let mut seen = HashSet::new();
    capabilities
        .into_iter()
        .filter(|capability| seen.insert((capability.id.clone(), capability.scope.clone())))
        .collect()
}

fn platform_matrix_for(capabilities: &[CapabilityFormula]) -> PlatformMatrix {
    let degraded = if capabilities
        .iter()
        .any(|capability| matches!(capability.id.as_str(), "fs.write" | "process.exec"))
    {
        vec!["web".to_string()]
    } else {
        Vec::new()
    };
    PlatformMatrix {
        supported: vec![
            "macos".to_string(),
            "windows".to_string(),
            "linux".to_string(),
            "web".to_string(),
            "remote".to_string(),
        ],
        degraded,
        unsupported: Vec::new(),
        reason: "Generated by platform.plugin-factory; refine before publishing.".to_string(),
    }
}

fn menu_for(id: &str, display_name: &str, route_path: &str) -> MenuContribution {
    let route_tail = route_path.trim_start_matches('/').replace('/', ":");
    let parent_permission_id = if route_path.starts_with("/assets/") {
        Some("perm-notes".to_string())
    } else if route_path.starts_with("/system/") {
        Some("perm-system".to_string())
    } else {
        None
    };
    MenuContribution {
        permission_id: format!("perm-{}", id.replace(['.', '_'], "-")),
        parent_permission_id,
        code: route_tail,
        title: display_name.to_string(),
        permission_type: "menu".to_string(),
        path: route_path.to_string(),
        component: format!("{route_path}/index"),
        icon: "lucide:puzzle".to_string(),
        sort_order: 900,
        ordinary_user: false,
    }
}

fn infer_view_schema(prompt: &str) -> String {
    if contains_any(prompt, &["表格", "列表", "table", "list"]) {
        "table".to_string()
    } else if contains_any(prompt, &["表单", "form"]) {
        "form".to_string()
    } else if contains_any(prompt, &["树", "tree"]) {
        "tree".to_string()
    } else {
        "markdown".to_string()
    }
}

fn category_for(id: &str) -> String {
    id.rsplit('.').next().unwrap_or(id).replace('-', "_")
}

fn slug_segment(value: &str) -> Option<String> {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if matches!(character, '-' | '_' | ' ' | '.' | '/' | ':') && !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

fn short_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    hash.chars().take(8).collect()
}

fn hash_directory(path: &Path) -> AppResult<String> {
    let mut hasher = Sha256::new();
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry.map_err(|error| {
            AppError::BadRequest(format!("扫描插件目录失败：{}: {error}", path.display()))
        })?;
        if entry.file_type().is_file() {
            let relative = entry
                .path()
                .strip_prefix(path)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            if matches!(
                relative.as_str(),
                "audit.jsonl"
                    | "diagnostics.json"
                    | "lock.json"
                    | "signature.json"
                    | "verification.json"
            ) {
                continue;
            }
            files.push(entry.into_path());
        }
    }
    files.sort();

    for file in files {
        let relative = file.strip_prefix(path).unwrap_or(&file);
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(fs::read(&file)?);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    let lower = value.to_lowercase();
    needles
        .iter()
        .any(|needle| lower.contains(&needle.to_lowercase()))
}

fn generated_files() -> Vec<String> {
    vec![
        "formula.json".to_string(),
        "permission-plan.md".to_string(),
        "context-pack.md".to_string(),
        "PLAN.md".to_string(),
        "README.md".to_string(),
        "src/index.ts".to_string(),
        "tests/smoke.test.ts".to_string(),
    ]
}

fn ensure_output_dir(output_dir: &Path, force: bool) -> AppResult<()> {
    if output_dir.exists() {
        if !output_dir.is_dir() {
            return Err(AppError::BadRequest(format!(
                "plugin draft output is not a directory: {}",
                output_dir.display()
            )));
        }
        if !force && fs::read_dir(output_dir)?.next().is_some() {
            return Err(AppError::Conflict(format!(
                "plugin draft output directory is not empty: {}",
                output_dir.display()
            )));
        }
    }
    fs::create_dir_all(output_dir)?;
    Ok(())
}

fn write_managed_file(path: &Path, content: &str, force: bool) -> AppResult<()> {
    if path.exists() && !force {
        return Err(AppError::Conflict(format!(
            "plugin draft file already exists: {}",
            path.display()
        )));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn permission_plan_markdown(draft: &PluginDraft) -> String {
    let mut output = format!("# Permission Plan\n\nPlugin: `{}`\n\n", draft.formula.id);
    if draft.permission_plan.is_empty() {
        output.push_str("No sensitive capabilities requested.\n");
        return output;
    }
    output.push_str("| Capability | Scope | Optional | Reason |\n");
    output.push_str("|---|---|---:|---|\n");
    for item in &draft.permission_plan {
        output.push_str(&format!(
            "| `{}` | `{}` | {} | {} |\n",
            item.id,
            item.scope,
            if item.optional { "yes" } else { "no" },
            item.reason.replace('|', "\\|")
        ));
    }
    output
}

fn context_pack_markdown(draft: &PluginDraft) -> String {
    let formula = &draft.formula;
    let mut output = format!(
        "# Context Pack\n\n## Intent\n{}\n\n## Formula Summary\n- ID: `{}`\n- Kind: `{}`\n- Display: {}\n\n",
        formula.intent, formula.id, formula.kind, formula.display_name
    );
    output.push_str("## Contributions\n");
    for command in &formula.contributes.commands {
        output.push_str(&format!("- Command `{}`: {}\n", command.id, command.title));
    }
    for view in &formula.contributes.views {
        output.push_str(&format!(
            "- View `{}`: `{}` at `{}`\n",
            view.id, view.schema, view.path
        ));
    }
    output.push_str("\n## Permission Plan\n");
    if draft.permission_plan.is_empty() {
        output.push_str("- No sensitive capabilities requested.\n");
    } else {
        for item in &draft.permission_plan {
            output.push_str(&format!(
                "- `{}` on `{}`: {}\n",
                item.id, item.scope, item.reason
            ));
        }
    }
    output.push_str("\n## File Map\n- `formula.json`: source of truth\n- `src/index.ts`: generated lifecycle stub\n- `tests/smoke.test.ts`: formula smoke test\n- `permission-plan.md`: capability review\n- `PLAN.md`: agent checklist\n\n## Recent Diagnostics\n- Generated draft has not been hot-loaded yet.\n\n## Checklist\n- [ ] Review formula identity and route\n- [ ] Confirm permission plan\n- [ ] Implement command behavior\n- [ ] Run schema validation\n- [ ] Publish to local registry\n");
    output
}

fn plan_markdown() -> String {
    "# Plugin Development Plan\n\n- [ ] 更新 formula.json\n- [ ] 生成或修复 src/index.ts\n- [ ] 补齐权限 fallback\n- [ ] 跑 schema 校验\n- [ ] 跑 smoke test\n- [ ] 更新 README\n- [ ] 发布到 local registry\n".to_string()
}

fn readme_markdown(draft: &PluginDraft) -> String {
    format!(
        "# {}\n\n{}\n\nThis capsule was generated by `platform.plugin-factory` from `plugin.createFromPrompt`.\n\nStart with `formula.json`, then implement `src/index.ts`, then run the smoke test and registry validation.\n",
        draft.formula.display_name, draft.formula.intent
    )
}

fn source_stub(draft: &PluginDraft) -> String {
    let plugin_id = json_string_literal(&draft.formula.id);
    let display_name = json_string_literal(&draft.formula.display_name);
    let commands = draft
        .formula
        .contributes
        .commands
        .iter()
        .map(|command| json_string_literal(&command.id))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "export interface PluginApiSnapshot {{\n  action: string;\n  entryPath: string;\n  hostKind: string;\n  pluginId: string;\n  schemaVersion: 'extension-host-api/v1';\n  sourcePath: string;\n  supportedCapabilities: string[];\n}}\n\nexport interface PluginApi {{\n  schemaVersion: 'extension-host-api/v1';\n  capabilities: {{\n    invoke(capability: string, input?: unknown): {{\n      capability: string;\n      hostKind: string;\n      input: unknown;\n      pluginId: string;\n      reason: string;\n      status: 'unsupported' | string;\n    }};\n  }};\n  events: {{\n    publish(eventType: string, payload?: unknown): {{\n      eventType: string;\n      hostKind: string;\n      payload: unknown;\n      pluginId: string;\n      status: 'recorded' | string;\n    }};\n    request(eventType: string, payload?: unknown): {{\n      eventType: string;\n      hostKind: string;\n      payload: unknown;\n      pluginId: string;\n      status: 'recorded' | string;\n    }};\n    stream(eventType: string, payload?: unknown, sequence?: number, done?: boolean): {{\n      eventType: string;\n      hostKind: string;\n      payload: unknown;\n      pluginId: string;\n      sequence?: number;\n      status: 'recorded' | string;\n    }};\n  }};\n  host: {{\n    describe(): PluginApiSnapshot;\n    snapshot(): PluginApiSnapshot;\n  }};\n}}\n\nexport interface PluginContext {{\n  api?: PluginApi;\n  hostKind?: string;\n  log?: (message: string) => void;\n  pluginId?: string;\n}}\n\nexport const pluginId = {plugin_id};\nexport const displayName = {display_name};\nexport const commands = [{commands}];\n\nexport async function activate(context: PluginContext = {{}}) {{\n  context.log?.(`${{displayName}} activated`);\n  context.api?.host.describe();\n  return {{ pluginId, commands }};\n}}\n\nexport async function deactivate(context: PluginContext = {{}}) {{\n  context.log?.(`${{displayName}} deactivated`);\n}}\n\nexport async function dispose(context: PluginContext = {{}}) {{\n  context.log?.(`${{displayName}} disposed`);\n}}\n"
    )
}

fn smoke_test_stub(draft: &PluginDraft) -> String {
    let plugin_id = json_string_literal(&draft.formula.id);
    format!(
        "import assert from 'node:assert/strict';\nimport {{ readFileSync }} from 'node:fs';\nimport {{ test }} from 'node:test';\n\nconst formula = JSON.parse(readFileSync(new URL('../formula.json', import.meta.url), 'utf8'));\n\ntest('formula identity is stable', () => {{\n  assert.equal(formula.schemaVersion, 'plugin-formula/v1');\n  assert.equal(formula.id, {plugin_id});\n  assert.ok(Array.isArray(formula.contributes?.commands));\n}});\n"
    )
}

fn json_string_literal(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        create_from_prompt, publish_gate, publish_local, repair_from_diagnostics,
        verify_plugin_draft, write_plugin_draft, PluginCreateFromPromptInput,
        PluginDiagnosticPermissionError, PluginDiagnosticPlatformError, PluginDiagnosticsPackage,
        PluginRepairFromDiagnosticsInput, PluginVerifyDraftInput, PLUGIN_DRAFT_SCHEMA_VERSION,
    };
    use std::fs;

    #[test]
    fn create_from_prompt_infers_capabilities_from_prompt() {
        let draft = create_from_prompt(PluginCreateFromPromptInput {
            prompt: "生成一个 Git 变更摘要插件，需要读取文件并执行 git status".to_string(),
            ..PluginCreateFromPromptInput::default()
        })
        .expect("draft");

        assert_eq!(draft.schema_version, PLUGIN_DRAFT_SCHEMA_VERSION);
        assert!(draft
            .formula
            .capabilities
            .iter()
            .any(|capability| capability.id == "process.exec"));
        assert!(draft
            .formula
            .capabilities
            .iter()
            .any(|capability| capability.id == "fs.read"));
    }

    #[test]
    fn create_from_prompt_can_generate_child_plugin_draft() {
        let draft = create_from_prompt(PluginCreateFromPromptInput {
            prompt: "资产表格插件，管理本地配置记录".to_string(),
            kind: Some("child-plugin".to_string()),
            route_path: Some("/assets/local-config".to_string()),
            ..PluginCreateFromPromptInput::default()
        })
        .expect("draft");

        assert_eq!(draft.formula.kind, "child-plugin");
        assert_eq!(
            draft
                .formula
                .parent
                .as_ref()
                .map(|parent| parent.plugin_id.as_str()),
            Some("asset-suite")
        );
    }

    #[test]
    fn write_plugin_draft_creates_capsule_files() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let draft = create_from_prompt(PluginCreateFromPromptInput {
            prompt: "HTTP API inspector plugin".to_string(),
            id: Some("workspace.http-inspector".to_string()),
            ..PluginCreateFromPromptInput::default()
        })
        .expect("draft");

        let result = write_plugin_draft(&draft, tempdir.path(), false).expect("write draft");

        assert_eq!(result.files.len(), 7);
        assert!(tempdir.path().join("formula.json").is_file());
        assert!(tempdir.path().join("permission-plan.md").is_file());
        assert!(tempdir.path().join("src/index.ts").is_file());

        let formula = fs::read_to_string(tempdir.path().join("formula.json")).expect("formula");
        assert!(formula.contains("workspace.http-inspector"));
    }

    #[test]
    fn publish_local_should_install_generated_draft() {
        let source_dir = tempfile::tempdir().expect("source tempdir");
        let data_dir = tempfile::tempdir().expect("data tempdir");
        let draft = create_from_prompt(PluginCreateFromPromptInput {
            prompt: "一个可以落地到本地 registry 的插件".to_string(),
            id: Some("workspace.publishable".to_string()),
            ..PluginCreateFromPromptInput::default()
        })
        .expect("draft");

        write_plugin_draft(&draft, source_dir.path(), true).expect("write draft");
        let installed = publish_local(source_dir.path(), data_dir.path()).expect("publish local");

        assert_eq!(installed.id, "workspace.publishable");
        assert!(source_dir.path().join("lock.json").is_file());
        assert!(source_dir.path().join("signature.json").is_file());
        assert!(source_dir.path().join("audit.jsonl").is_file());
        assert!(data_dir
            .path()
            .join("plugin-registry/installed.json")
            .is_file());
    }

    #[test]
    fn publish_gate_should_write_package_gate_artifacts() {
        let source_dir = tempfile::tempdir().expect("source tempdir");
        let data_dir = tempfile::tempdir().expect("data tempdir");
        let draft = create_from_prompt(PluginCreateFromPromptInput {
            prompt: "发布门禁插件，需要表格视图".to_string(),
            id: Some("workspace.publish-gate".to_string()),
            ..PluginCreateFromPromptInput::default()
        })
        .expect("draft");
        write_plugin_draft(&draft, source_dir.path(), true).expect("write draft");

        let report = publish_gate(source_dir.path(), data_dir.path(), true).expect("publish gate");

        assert_eq!(report.status, "passed");
        assert_eq!(report.plugin_id, "workspace.publish-gate");
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "signature.placeholder"));
        assert!(source_dir.path().join("verification.json").is_file());
        assert!(source_dir.path().join("lock.json").is_file());
        assert!(source_dir.path().join("signature.json").is_file());
        assert!(data_dir
            .path()
            .join("plugin-registry/remote-registry.protocol.json")
            .is_file());
    }

    #[test]
    fn repair_from_diagnostics_should_patch_git_and_web_fallback() {
        let source_dir = tempfile::tempdir().expect("source tempdir");
        let draft = create_from_prompt(PluginCreateFromPromptInput {
            prompt: "生成一个 Git 变更摘要插件".to_string(),
            id: Some("workspace.repairable".to_string()),
            ..PluginCreateFromPromptInput::default()
        })
        .expect("draft");
        write_plugin_draft(&draft, source_dir.path(), true).expect("write draft");

        let diagnostics_path = source_dir.path().join("diagnostics.json");
        let diagnostics = PluginDiagnosticsPackage {
            plugin_id: draft.formula.id.clone(),
            run_id: "run-001".to_string(),
            status: "failed".to_string(),
            formula_errors: Vec::new(),
            permission_errors: vec![PluginDiagnosticPermissionError {
                capability: "process.exec".to_string(),
                reason: "allow list missing git".to_string(),
            }],
            platform_errors: vec![PluginDiagnosticPlatformError {
                platform: "web".to_string(),
                reason: "process.exec unavailable; fallback missing".to_string(),
            }],
            test_failures: Vec::new(),
            ui_preview: None,
            repair_hint: "Add web fallback and declare git in process.exec allow list.".to_string(),
            source_path: Some(source_dir.path().to_string_lossy().into_owned()),
        };
        fs::write(
            &diagnostics_path,
            serde_json::to_string_pretty(&diagnostics).expect("json"),
        )
        .expect("diagnostics");

        let result = repair_from_diagnostics(PluginRepairFromDiagnosticsInput {
            diagnostics_path: diagnostics_path.to_string_lossy().into_owned(),
            source_path: None,
            force: true,
        })
        .expect("repair");

        assert_eq!(result.draft.formula.id, "workspace.repairable");
        let formula_text =
            fs::read_to_string(source_dir.path().join("formula.json")).expect("formula");
        assert!(formula_text.contains("\"git\""));
        assert!(formula_text.contains("\"web\""));
    }

    #[test]
    fn verify_plugin_draft_should_write_diagnostics_and_verification_report() {
        let source_dir = tempfile::tempdir().expect("source tempdir");
        let draft = create_from_prompt(PluginCreateFromPromptInput {
            prompt: "生成一个 Git 变更摘要插件，需要读取文件并执行 git status".to_string(),
            id: Some("workspace.verifiable".to_string()),
            ..PluginCreateFromPromptInput::default()
        })
        .expect("draft");
        write_plugin_draft(&draft, source_dir.path(), true).expect("write draft");

        let report = verify_plugin_draft(PluginVerifyDraftInput {
            source_path: source_dir.path().to_string_lossy().into_owned(),
            output_dir: None,
            write: true,
        })
        .expect("verify");

        assert_eq!(report.status, "passed");
        assert_eq!(report.plugin_id, "workspace.verifiable");
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "permission.plan" && check.status == "passed"));
        assert!(report
            .checks
            .iter()
            .any(|check| check.id == "ui.preview" && check.status == "passed"));
        let preview = report.diagnostics.ui_preview.as_ref().expect("ui preview");
        let dom_snapshot = preview
            .dom_snapshots
            .iter()
            .find(|snapshot| snapshot.id == "formula-view-preview")
            .expect("dom snapshot");
        assert!(dom_snapshot.html.contains("data-aio-plugin-preview"));
        assert!(dom_snapshot
            .path
            .as_deref()
            .is_some_and(|path| path.ends_with("ui-preview.formula-view-preview.dom.html")));
        assert!(source_dir
            .path()
            .join("ui-preview.formula-view-preview.dom.html")
            .is_file());
        assert!(source_dir.path().join("diagnostics.json").is_file());
        assert!(source_dir.path().join("verification.json").is_file());
    }

    #[test]
    fn verify_plugin_draft_should_report_missing_permission_plan() {
        let source_dir = tempfile::tempdir().expect("source tempdir");
        let draft = create_from_prompt(PluginCreateFromPromptInput {
            prompt: "生成一个需要执行 git status 的插件".to_string(),
            id: Some("workspace.bad-permission-plan".to_string()),
            ..PluginCreateFromPromptInput::default()
        })
        .expect("draft");
        write_plugin_draft(&draft, source_dir.path(), true).expect("write draft");
        fs::remove_file(source_dir.path().join("permission-plan.md")).expect("remove plan");

        let report = verify_plugin_draft(PluginVerifyDraftInput {
            source_path: source_dir.path().to_string_lossy().into_owned(),
            output_dir: None,
            write: false,
        })
        .expect("verify");

        assert_eq!(report.status, "failed");
        assert!(report
            .diagnostics
            .permission_errors
            .iter()
            .any(|issue| issue.capability == "process.exec"));
        assert!(report
            .diagnostics
            .repair_hint
            .contains("permission-plan.md"));
    }
}
