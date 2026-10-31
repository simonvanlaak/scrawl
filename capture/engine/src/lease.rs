use crate::Runtime;
use std::io;
#[derive(Clone, Copy)]
pub enum LeaseKind {
    Read,
    Write,
    Finalize,
    Maintenance,
}
/// Identity is a persistent bundle ID, never a pathname. Read/write/finalize coexist;
/// this is a maintenance exclusion mechanism, not a concurrent-writer coordinator.
pub struct BundleLease {
    pub(crate) lock: crate::locking::Lock,
}
impl Runtime {
    pub fn lease(&self, id: &str, kind: LeaseKind) -> io::Result<Option<BundleLease>> {
        crate::identity::validate(id, 32)?;
        let _tx = self.transaction()?;
        self.lease_locked(id, kind)
    }
    pub(crate) fn lease_locked(
        &self,
        id: &str,
        kind: LeaseKind,
    ) -> io::Result<Option<BundleLease>> {
        let lock = crate::locking::Lock::open(&self.store, &format!("bundle-{id}.lock"))?;
        Ok(lock
            .attempt(&self.store, !matches!(kind, LeaseKind::Maintenance))?
            .then_some(BundleLease { lock }))
    }
}
