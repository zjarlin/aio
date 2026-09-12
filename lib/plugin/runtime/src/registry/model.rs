use az_plugin_bundle::Bundle;
use az_plugin_contract::CapabilityGrants;

pub struct StoredRelease {
    pub generation: i64,
    pub bundle: Bundle,
    pub grants: CapabilityGrants,
}
