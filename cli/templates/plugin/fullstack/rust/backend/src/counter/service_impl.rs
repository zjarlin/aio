use anyhow::{Context as _, Result};
use fullstack_model::{IncrementRequest, IncrementResponse};

use super::service::CounterService;

pub(super) struct Counter;

impl CounterService for Counter {
    fn increment(&self, request: IncrementRequest, tenant_id: String) -> Result<IncrementResponse> {
        Ok(IncrementResponse {
            count: request.count.checked_add(1).context("计数超过上限")?,
            tenant_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increments_with_host_tenant_and_rejects_overflow() -> Result<()> {
        let result = Counter.increment(IncrementRequest { count: 41 }, "tenant-a".to_owned())?;
        assert_eq!(result.count, 42);
        assert_eq!(result.tenant_id, "tenant-a");
        assert!(
            Counter
                .increment(IncrementRequest { count: i64::MAX }, "tenant-a".to_owned())
                .is_err()
        );
        Ok(())
    }
}
