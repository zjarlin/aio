use anyhow::Result;
use fullstack_model::{IncrementRequest, IncrementResponse};

pub(super) trait CounterService: Send + Sync {
    fn increment(&self, request: IncrementRequest, tenant_id: String) -> Result<IncrementResponse>;
}
