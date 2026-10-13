use crate::call_log::StateKind;

#[derive(Default)]
pub(crate) struct StateCounts {
    running: usize,
    waiting_at_gate: usize,
    hanging: usize,
    succeeded: usize,
    failed: usize,
    dropped: usize,
}

impl StateCounts {
    pub(crate) fn get(&self, kind: StateKind) -> usize {
        match kind {
            StateKind::Running => self.running,
            StateKind::WaitingAtGate => self.waiting_at_gate,
            StateKind::Hanging => self.hanging,
            StateKind::Succeeded => self.succeeded,
            StateKind::Failed => self.failed,
            StateKind::Dropped => self.dropped,
        }
    }

    pub(crate) fn enter(&mut self, kind: StateKind) {
        let count = self.counter(kind);
        *count = count.saturating_add(1);
    }

    pub(crate) fn leave(&mut self, kind: StateKind) {
        let count = self.counter(kind);
        *count = count.saturating_sub(1);
    }

    fn counter(&mut self, kind: StateKind) -> &mut usize {
        match kind {
            StateKind::Running => &mut self.running,
            StateKind::WaitingAtGate => &mut self.waiting_at_gate,
            StateKind::Hanging => &mut self.hanging,
            StateKind::Succeeded => &mut self.succeeded,
            StateKind::Failed => &mut self.failed,
            StateKind::Dropped => &mut self.dropped,
        }
    }
}
