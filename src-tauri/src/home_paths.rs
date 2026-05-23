pub(crate) const DEFAULT_HOME_PATH: &str = "/analytics";
pub(crate) const ORDINARY_USER_HOME_PATH: &str = "/assets/notes";

const ORDINARY_USER_ROLE: &str = "ordinary_user";
const SUPER_ADMIN_ROLE: &str = "super_admin";

pub(crate) fn home_path_for_roles(home_path: &str, roles: &[String]) -> String {
    let normalized = home_path.trim();
    if is_ordinary_user(roles) && (normalized.is_empty() || normalized == DEFAULT_HOME_PATH) {
        return ORDINARY_USER_HOME_PATH.to_string();
    }
    if normalized.is_empty() {
        return DEFAULT_HOME_PATH.to_string();
    }
    normalized.to_string()
}

fn is_ordinary_user(roles: &[String]) -> bool {
    roles.iter().any(|role| role == ORDINARY_USER_ROLE)
        && !roles.iter().any(|role| role == SUPER_ADMIN_ROLE)
}

#[cfg(test)]
mod tests {
    use super::{home_path_for_roles, DEFAULT_HOME_PATH, ORDINARY_USER_HOME_PATH};

    #[test]
    fn ordinary_user_legacy_default_home_path_maps_to_assets() {
        let roles = vec!["ordinary_user".to_string()];
        assert_eq!(
            home_path_for_roles(DEFAULT_HOME_PATH, &roles),
            ORDINARY_USER_HOME_PATH,
        );
    }

    #[test]
    fn ordinary_user_custom_home_path_is_preserved() {
        let roles = vec!["ordinary_user".to_string()];
        assert_eq!(home_path_for_roles("/workspace", &roles), "/workspace");
    }

    #[test]
    fn super_admin_keeps_default_home_path() {
        let roles = vec!["ordinary_user".to_string(), "super_admin".to_string()];
        assert_eq!(
            home_path_for_roles(DEFAULT_HOME_PATH, &roles),
            DEFAULT_HOME_PATH,
        );
    }
}
