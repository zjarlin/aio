use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::now_millis,
    error::{AppError, AppResult},
    plugin_registry::{PlatformMatrix, RegistryPolicyRecord},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDecisionRecord {
    pub trace_id: String,
    pub user_id: String,
    pub source_id: String,
    pub source_kind: String,
    pub capability: String,
    pub scope: String,
    pub target: String,
    pub decision: String,
    pub reason: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRuntimeRequest {
    pub user_id: String,
    pub source_id: String,
    pub source_kind: String,
    pub capability: String,
    pub target: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub declared_capabilities: Vec<String>,
    #[serde(default)]
    pub declared_scopes: Vec<String>,
    #[serde(default)]
    pub platform_supported: Vec<String>,
    #[serde(default)]
    pub platform_degraded: Vec<String>,
    #[serde(default)]
    pub platform_unsupported: Vec<String>,
    #[serde(default)]
    pub consent_granted: bool,
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Default)]
pub struct PermissionCore {
    audit_log: Arc<Mutex<Vec<PermissionDecisionRecord>>>,
}

impl PermissionCore {
    pub fn audit_log(&self) -> Vec<PermissionDecisionRecord> {
        self.audit_log
            .lock()
            .map(|entries| entries.clone())
            .unwrap_or_default()
    }

    pub fn evaluate_runtime(
        &self,
        request: PermissionRuntimeRequest,
    ) -> AppResult<PermissionDecisionRecord> {
        self.evaluate_runtime_with_policies(request, &[])
    }

    pub fn evaluate_runtime_with_policies(
        &self,
        request: PermissionRuntimeRequest,
        policies: &[RegistryPolicyRecord],
    ) -> AppResult<PermissionDecisionRecord> {
        let decision = self.decide(&request, policies);
        self.push_audit(decision.clone());
        if decision.decision == "allow" {
            Ok(decision)
        } else {
            Err(AppError::Forbidden)
        }
    }

    pub fn system_request(
        user_id: impl Into<String>,
        source_id: impl Into<String>,
        capability: impl Into<String>,
        target: impl Into<String>,
        declared_capabilities: impl IntoIterator<Item = impl Into<String>>,
    ) -> PermissionRuntimeRequest {
        let capability = capability.into();
        PermissionRuntimeRequest {
            user_id: user_id.into(),
            source_id: source_id.into(),
            source_kind: "system".to_string(),
            target: target.into(),
            scope: "*".to_string(),
            declared_capabilities: declared_capabilities.into_iter().map(Into::into).collect(),
            declared_scopes: vec!["*".to_string()],
            platform_supported: vec![current_platform().to_string()],
            platform_degraded: Vec::new(),
            platform_unsupported: Vec::new(),
            consent_granted: true,
            reason: "trusted platform command".to_string(),
            capability,
        }
    }

    fn decide(
        &self,
        request: &PermissionRuntimeRequest,
        policies: &[RegistryPolicyRecord],
    ) -> PermissionDecisionRecord {
        let (decision, reason) = match runtime_denial_reason(request) {
            Some(reason) => ("deny", reason),
            None => match policy_denial_reason(request, policies) {
                Some(reason) => ("deny", reason),
                None => (
                    "allow",
                    "manifest, platform, consent and policy checks passed".to_string(),
                ),
            },
        };

        PermissionDecisionRecord {
            trace_id: Uuid::new_v4().to_string(),
            user_id: request.user_id.clone(),
            source_id: request.source_id.clone(),
            source_kind: request.source_kind.clone(),
            capability: request.capability.clone(),
            scope: request.scope.clone(),
            target: request.target.clone(),
            decision: decision.to_string(),
            reason,
            timestamp: now_millis(),
        }
    }

    fn push_audit(&self, record: PermissionDecisionRecord) {
        if let Ok(mut entries) = self.audit_log.lock() {
            entries.push(record);
        }
    }

    pub fn validate_schema_version(
        source_id: &str,
        expected: &str,
        actual: &str,
        diagnostics: &mut Vec<String>,
    ) {
        if actual != expected {
            diagnostics.push(format!(
                "{source_id} has invalid schemaVersion {actual}, expected {expected}"
            ));
        }
    }

    pub fn validate_platform_values(values: &[String], path: &str, diagnostics: &mut Vec<String>) {
        for platform in values {
            if !matches!(
                platform.as_str(),
                "macos" | "windows" | "linux" | "web" | "remote"
            ) {
                diagnostics.push(format!("unknown platform {platform} at {path}"));
            }
        }
    }

    pub fn validate_platform_lists(
        source_id: &str,
        supported: &[String],
        degraded: &[String],
        unsupported: &[String],
        diagnostics: &mut Vec<String>,
    ) {
        Self::validate_platform_uniqueness(source_id, "supported", supported, diagnostics);
        Self::validate_platform_uniqueness(source_id, "degraded", degraded, diagnostics);
        Self::validate_platform_uniqueness(source_id, "unsupported", unsupported, diagnostics);
        Self::validate_platform_values(
            supported,
            &format!("{source_id}.platforms.supported"),
            diagnostics,
        );
        Self::validate_platform_values(
            degraded,
            &format!("{source_id}.platforms.degraded"),
            diagnostics,
        );
        Self::validate_platform_values(
            unsupported,
            &format!("{source_id}.platforms.unsupported"),
            diagnostics,
        );
        Self::validate_platform_conflicts(source_id, supported, degraded, unsupported, diagnostics);
    }

    pub fn validate_declared_capabilities(
        source_id: &str,
        contribution_kind: &str,
        contribution_id: &str,
        required_capabilities: &[String],
        declared_capabilities: &HashSet<&str>,
        diagnostics: &mut Vec<String>,
    ) {
        for capability in required_capabilities {
            if !declared_capabilities.contains(capability.as_str()) {
                diagnostics.push(format!(
                    "{} {}.{} requires undeclared capability {}",
                    source_id, contribution_kind, contribution_id, capability
                ));
            }
        }
    }

    pub fn validate_event_names(
        source_id: &str,
        list_name: &str,
        events: &[String],
        diagnostics: &mut Vec<String>,
    ) {
        let mut seen = HashSet::new();
        for event in events {
            let event = event.trim();
            if event.is_empty() {
                diagnostics.push(format!(
                    "{source_id}.events.{list_name} contains an empty event name"
                ));
                continue;
            }
            if !seen.insert(event) {
                diagnostics.push(format!(
                    "{source_id}.events.{list_name} contains duplicate event {event}"
                ));
            }
        }
    }

    pub fn validate_when_expression(
        source_id: &str,
        contribution_kind: &str,
        contribution_id: &str,
        when: &str,
        platforms: Option<&PlatformMatrix>,
        available_capabilities: &HashSet<&str>,
        diagnostics: &mut Vec<String>,
    ) {
        let when = when.trim();
        if when.is_empty() {
            return;
        }

        for clause in when.split("&&") {
            let clause = clause.trim();
            if clause.is_empty() {
                diagnostics.push(format!(
                    "{} {}.{} has an empty when clause",
                    source_id, contribution_kind, contribution_id
                ));
                continue;
            }

            match parse_when_clause(clause) {
                Ok(WhenClause::Platform(platform)) => {
                    Self::validate_platform_values(
                        std::slice::from_ref(&platform),
                        &format!("{source_id}.{contribution_kind}.{contribution_id}.when"),
                        diagnostics,
                    );
                    if let Some(matrix) = platforms {
                        if matrix
                            .unsupported
                            .iter()
                            .any(|candidate| candidate == &platform)
                        {
                            diagnostics.push(format!(
                                "{} {}.{} when requires platform {} but source.platforms.unsupported includes it",
                                source_id, contribution_kind, contribution_id, platform
                            ));
                        }
                        if !matrix.supported.is_empty()
                            && !matrix
                                .supported
                                .iter()
                                .any(|candidate| candidate == &platform)
                            && !matrix
                                .degraded
                                .iter()
                                .any(|candidate| candidate == &platform)
                        {
                            diagnostics.push(format!(
                                "{} {}.{} when requires platform {} but source.platforms does not include it",
                                source_id, contribution_kind, contribution_id, platform
                            ));
                        }
                    }
                }
                Ok(WhenClause::Capability(capability)) => {
                    if !available_capabilities.contains(capability.as_str()) {
                        diagnostics.push(format!(
                            "{} {}.{} when references unknown capability {}",
                            source_id, contribution_kind, contribution_id, capability
                        ));
                    }
                }
                Ok(WhenClause::Workspace) | Ok(WhenClause::TrustLevel) => {}
                Err(error) => diagnostics.push(format!(
                    "{} {}.{} has unsupported when clause `{clause}`: {error}",
                    source_id, contribution_kind, contribution_id
                )),
            }
        }
    }

    pub fn validate_permission_parent_ids(
        permissions: &[(String, Option<String>)],
        diagnostics: &mut Vec<String>,
    ) {
        let permission_ids = permissions
            .iter()
            .map(|(permission_id, _)| permission_id.as_str())
            .collect::<HashSet<_>>();

        for (permission_id, parent_permission_id) in permissions {
            let Some(parent_id) = parent_permission_id.as_deref() else {
                continue;
            };
            if !permission_ids.contains(parent_id) {
                diagnostics.push(format!(
                    "permission {} references unknown parent {}",
                    permission_id, parent_id
                ));
            }
        }
    }

    fn validate_platform_uniqueness(
        source_id: &str,
        list_name: &str,
        platforms: &[String],
        diagnostics: &mut Vec<String>,
    ) {
        let mut seen = HashSet::new();
        for platform in platforms {
            if !seen.insert(platform) {
                diagnostics.push(format!(
                    "{source_id}.platforms.{list_name} contains duplicate platform {platform}"
                ));
            }
        }
    }

    fn validate_platform_conflicts(
        source_id: &str,
        supported: &[String],
        degraded: &[String],
        unsupported: &[String],
        diagnostics: &mut Vec<String>,
    ) {
        let supported_set = supported.iter().collect::<HashSet<_>>();
        let degraded_set = degraded.iter().collect::<HashSet<_>>();
        let unsupported_set = unsupported.iter().collect::<HashSet<_>>();

        for platform in supported_set.intersection(&degraded_set) {
            diagnostics.push(format!(
                "{source_id}.platforms lists platform {platform} as both supported and degraded"
            ));
        }
        for platform in supported_set.intersection(&unsupported_set) {
            diagnostics.push(format!(
                "{source_id}.platforms lists platform {platform} as both supported and unsupported"
            ));
        }
        for platform in degraded_set.intersection(&unsupported_set) {
            diagnostics.push(format!(
                "{source_id}.platforms lists platform {platform} as both degraded and unsupported"
            ));
        }
    }
}

fn runtime_denial_reason(request: &PermissionRuntimeRequest) -> Option<String> {
    let source_id = request.source_id.trim();
    if source_id.is_empty() {
        return Some("source_id is required".to_string());
    }

    let capability = request.capability.trim();
    if capability.is_empty() {
        return Some("capability is required".to_string());
    }

    if !request
        .declared_capabilities
        .iter()
        .any(|declared| declared == capability)
    {
        return Some(format!(
            "{source_id} did not declare required capability {capability}"
        ));
    }

    let scope = request.scope.trim();
    if scope.is_empty() {
        return Some(format!("{source_id} request is missing capability scope"));
    }
    if !request
        .declared_scopes
        .iter()
        .any(|declared| scope_allowed(declared, scope))
    {
        return Some(format!(
            "{source_id} requested scope {scope} outside declared capability scopes"
        ));
    }

    let platform = current_platform();
    if request
        .platform_unsupported
        .iter()
        .any(|unsupported| unsupported == platform)
    {
        return Some(format!("{source_id} does not support platform {platform}"));
    }

    let platform_supported = request
        .platform_supported
        .iter()
        .any(|supported| supported == platform);
    let platform_degraded = request
        .platform_degraded
        .iter()
        .any(|degraded| degraded == platform);
    if !request.platform_supported.is_empty() && !platform_supported && !platform_degraded {
        return Some(format!(
            "{source_id} has no supported or degraded runtime path for platform {platform}"
        ));
    }

    if !request.consent_granted {
        return Some(format!(
            "{source_id} has no granted consent for capability {capability}"
        ));
    }

    None
}

fn policy_denial_reason(
    request: &PermissionRuntimeRequest,
    policies: &[RegistryPolicyRecord],
) -> Option<String> {
    policies
        .iter()
        .filter(|policy| policy.effect == "deny")
        .filter(|policy| policy_matches(policy, request))
        .map(|policy| {
            if policy.reason.trim().is_empty() {
                format!(
                    "policy {} from {} denied capability {}",
                    policy.id, policy.source_id, request.capability
                )
            } else {
                format!(
                    "policy {} from {} denied capability {}: {}",
                    policy.id, policy.source_id, request.capability, policy.reason
                )
            }
        })
        .next()
}

fn policy_matches(policy: &RegistryPolicyRecord, request: &PermissionRuntimeRequest) -> bool {
    list_matches(&policy.source_ids, &request.source_id)
        && list_matches(&policy.source_kinds, &request.source_kind)
        && list_matches(&policy.capabilities, &request.capability)
        && scope_list_matches(&policy.scopes, &request.scope)
        && target_contains_matches(&policy.target_contains, &request.target)
        && platform_list_matches(&policy.platforms, current_platform())
}

fn list_matches(values: &[String], actual: &str) -> bool {
    values.is_empty()
        || values
            .iter()
            .any(|value| value == "*" || value.trim() == actual)
}

fn scope_list_matches(values: &[String], actual: &str) -> bool {
    values.is_empty()
        || values
            .iter()
            .any(|value| value == "*" || scope_allowed(value, actual))
}

fn target_contains_matches(values: &[String], actual: &str) -> bool {
    values.is_empty()
        || values
            .iter()
            .any(|value| !value.trim().is_empty() && actual.contains(value.trim()))
}

fn platform_list_matches(values: &[String], actual: &str) -> bool {
    values.is_empty()
        || values
            .iter()
            .any(|value| value == "*" || value.trim() == actual)
}

fn scope_allowed(declared_scope: &str, requested_scope: &str) -> bool {
    let declared_scope = declared_scope.trim();
    let requested_scope = requested_scope.trim();
    declared_scope == "*"
        || declared_scope == requested_scope
        || requested_scope.starts_with(&format!("{}/", declared_scope.trim_end_matches('/')))
}

fn current_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(all(
        target_os = "linux",
        not(target_os = "macos"),
        not(target_os = "windows")
    ))]
    {
        "linux"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        "remote"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WhenClause {
    Capability(String),
    Platform(String),
    TrustLevel,
    Workspace,
}

fn parse_when_clause(clause: &str) -> Result<WhenClause, String> {
    let clause = clause.trim();
    if clause.starts_with("platform") {
        return parse_when_value(clause, "platform").map(WhenClause::Platform);
    }
    if clause.starts_with("capability") {
        return parse_when_value(clause, "capability").map(WhenClause::Capability);
    }
    if clause.starts_with("workspace") {
        return Ok(WhenClause::Workspace);
    }
    if clause.starts_with("trustLevel") {
        return Ok(WhenClause::TrustLevel);
    }
    Err("unsupported condition".to_string())
}

fn parse_when_value(clause: &str, keyword: &str) -> Result<String, String> {
    let remainder = clause
        .strip_prefix(keyword)
        .ok_or_else(|| format!("missing {keyword} keyword"))?
        .trim_start();
    let value = if let Some(value) = remainder.strip_prefix(':') {
        value.trim()
    } else if let Some(value) = remainder.strip_prefix("==") {
        value.trim()
    } else if remainder.starts_with('(') && remainder.ends_with(')') {
        remainder[1..remainder.len() - 1].trim()
    } else {
        remainder.trim()
    };
    if value.is_empty() {
        return Err(format!("{keyword} condition is missing a value"));
    }
    Ok(unquote(value))
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
            || (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
        {
            return trimmed[1..trimmed.len() - 1].trim().to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{current_platform, PermissionCore, PermissionRuntimeRequest, PlatformMatrix};
    use crate::plugin_registry::RegistryPolicyRecord;

    fn runtime_request(capability: &str) -> PermissionRuntimeRequest {
        PermissionRuntimeRequest {
            user_id: "user-1".to_string(),
            source_id: "plugin.sample".to_string(),
            source_kind: "plugin".to_string(),
            capability: capability.to_string(),
            target: "sample-target".to_string(),
            scope: "sample-target".to_string(),
            declared_capabilities: vec![capability.to_string()],
            declared_scopes: vec!["sample-target".to_string()],
            platform_supported: vec![current_platform().to_string()],
            platform_degraded: Vec::new(),
            platform_unsupported: Vec::new(),
            consent_granted: true,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn evaluate_runtime_should_allow_declared_capability_with_consent() {
        let core = PermissionCore::default();

        let decision = core.evaluate_runtime(runtime_request("fs.read")).unwrap();

        assert_eq!(decision.decision, "allow");
        assert_eq!(core.audit_log().len(), 1);
    }

    #[test]
    fn evaluate_runtime_should_deny_undeclared_capability() {
        let core = PermissionCore::default();
        let mut request = runtime_request("fs.write");
        request.declared_capabilities = vec!["fs.read".to_string()];

        let result = core.evaluate_runtime(request);

        assert!(result.is_err());
        let audit = core.audit_log();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].decision, "deny");
        assert!(audit[0].reason.contains("did not declare"));
    }

    #[test]
    fn evaluate_runtime_should_deny_without_consent() {
        let core = PermissionCore::default();
        let mut request = runtime_request("clipboard.read");
        request.consent_granted = false;

        let result = core.evaluate_runtime(request);

        assert!(result.is_err());
        let audit = core.audit_log();
        assert_eq!(audit[0].decision, "deny");
        assert!(audit[0].reason.contains("no granted consent"));
    }

    #[test]
    fn evaluate_runtime_should_deny_scope_outside_manifest() {
        let core = PermissionCore::default();
        let mut request = runtime_request("fs.read");
        request.scope = "/private/tmp/outside.txt".to_string();
        request.declared_scopes = vec!["/Users/example/workspace".to_string()];

        let result = core.evaluate_runtime(request);

        assert!(result.is_err());
        let audit = core.audit_log();
        assert_eq!(audit[0].decision, "deny");
        assert!(audit[0]
            .reason
            .contains("outside declared capability scopes"));
    }

    #[test]
    fn evaluate_runtime_should_deny_unsupported_platform() {
        let core = PermissionCore::default();
        let mut request = runtime_request("notification.send");
        request.platform_supported = vec!["web".to_string()];
        request.platform_degraded = Vec::new();

        let result = core.evaluate_runtime(request);

        assert!(result.is_err());
        let audit = core.audit_log();
        assert_eq!(audit[0].decision, "deny");
        assert!(audit[0].reason.contains("no supported or degraded"));
    }

    #[test]
    fn evaluate_runtime_should_apply_deny_policy_after_base_checks() {
        let core = PermissionCore::default();
        let request = runtime_request("process.exec");
        let policies = vec![RegistryPolicyRecord {
            id: "policy.block-shell".to_string(),
            title: "Block Shell".to_string(),
            effect: "deny".to_string(),
            reason: "shell execution requires approval".to_string(),
            priority: 10,
            source_ids: vec!["plugin.sample".to_string()],
            source_kinds: Vec::new(),
            capabilities: vec!["process.exec".to_string()],
            scopes: Vec::new(),
            target_contains: vec!["sample".to_string()],
            platforms: vec![current_platform().to_string()],
            when: String::new(),
            source_id: "governance.high-risk-deny-policy".to_string(),
            source_kind: "child-plugin".to_string(),
            parent_chain: vec!["platform.permission-core".to_string()],
        }];

        let result = core.evaluate_runtime_with_policies(request, &policies);

        assert!(result.is_err());
        let audit = core.audit_log();
        assert_eq!(audit[0].decision, "deny");
        assert!(audit[0].reason.contains("policy.block-shell"));
    }

    #[test]
    fn evaluate_runtime_should_not_let_policy_bypass_base_checks() {
        let core = PermissionCore::default();
        let mut request = runtime_request("fs.write");
        request.declared_capabilities = vec!["fs.read".to_string()];
        let policies = vec![RegistryPolicyRecord {
            id: "policy.warn-only".to_string(),
            title: "Warn Only".to_string(),
            effect: "warn".to_string(),
            reason: "would warn".to_string(),
            priority: 10,
            source_ids: Vec::new(),
            source_kinds: Vec::new(),
            capabilities: vec!["fs.write".to_string()],
            scopes: Vec::new(),
            target_contains: Vec::new(),
            platforms: Vec::new(),
            when: String::new(),
            source_id: "governance.policy".to_string(),
            source_kind: "child-plugin".to_string(),
            parent_chain: vec!["platform.permission-core".to_string()],
        }];

        let result = core.evaluate_runtime_with_policies(request, &policies);

        assert!(result.is_err());
        let audit = core.audit_log();
        assert!(audit[0].reason.contains("did not declare"));
    }

    #[test]
    fn validate_schema_version_should_report_mismatch() {
        let mut diagnostics = Vec::new();

        PermissionCore::validate_schema_version(
            "asset-suite",
            "plugin-formula/v1",
            "plugin-formula/v2",
            &mut diagnostics,
        );

        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn validate_platform_lists_should_report_conflicts_and_duplicates() {
        let mut diagnostics = Vec::new();

        PermissionCore::validate_platform_lists(
            "asset-suite",
            &["macos".to_string(), "macos".to_string()],
            &["macos".to_string()],
            &["windows".to_string(), "windows".to_string()],
            &mut diagnostics,
        );

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("duplicate platform macos")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("both supported and degraded")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("duplicate platform windows")));
    }

    #[test]
    fn validate_when_expression_should_report_platform_and_capability_mismatch() {
        let mut diagnostics = Vec::new();
        let platforms = PlatformMatrix {
            supported: vec!["macos".to_string()],
            degraded: Vec::new(),
            unsupported: vec!["windows".to_string()],
            reason: String::new(),
        };
        let capabilities = HashSet::from(["clipboard.write"]);

        PermissionCore::validate_when_expression(
            "platform.macos-clipboard",
            "command",
            "macos_clipboard_copy",
            "platform == 'windows' && capability('clipboard.write') && capability('missing.cap')",
            Some(&platforms),
            &capabilities,
            &mut diagnostics,
        );

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("requires platform windows")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("unknown capability missing.cap")));
    }

    #[test]
    fn validate_event_names_should_report_empty_and_duplicate_events() {
        let mut diagnostics = Vec::new();

        PermissionCore::validate_event_names(
            "git-suite",
            "publishes",
            &[
                "git.changed".to_string(),
                "".to_string(),
                "git.changed".to_string(),
            ],
            &mut diagnostics,
        );

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("empty event name")));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("duplicate event git.changed")));
    }
}
