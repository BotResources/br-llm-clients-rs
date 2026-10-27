use std::collections::{HashMap, HashSet};

use br_llm_messages::{
    BlockId, ContentBlock, ProjectedThread, RuntimeToolCall, ToolName, Turn, TurnEntry,
};

pub(super) struct CallInfo {
    pub(super) wire_id: String,
    pub(super) tool_name: ToolName,
    pub(super) index: usize,
}

pub(super) struct CallTable {
    calls: HashMap<BlockId, CallInfo>,
}

impl CallTable {
    pub(super) fn of(thread: &ProjectedThread) -> Self {
        let calls = runtime_calls(thread);
        let external: HashSet<&str> = calls
            .iter()
            .filter_map(|call| call.external_id().map(|id| id.as_str()))
            .collect();
        let mut assigned: HashSet<String> = HashSet::new();
        let mut table = HashMap::new();
        for (index, call) in calls.iter().enumerate() {
            let wire_id = match call.external_id() {
                Some(id) if !assigned.contains(id.as_str()) => id.as_str().to_owned(),
                _ => generated(call, &assigned, &external),
            };
            assigned.insert(wire_id.clone());
            table.insert(
                call.id(),
                CallInfo {
                    wire_id,
                    tool_name: call.name().clone(),
                    index,
                },
            );
        }
        Self { calls: table }
    }

    pub(super) fn get(&self, call: BlockId) -> Option<&CallInfo> {
        self.calls.get(&call)
    }
}

fn runtime_calls(thread: &ProjectedThread) -> Vec<&RuntimeToolCall> {
    let mut calls = Vec::new();
    for turn in thread.turns() {
        let Turn::AITurn(ai) = turn else { continue };
        for entry in ai.entries() {
            let TurnEntry::ModelResponse(response) = entry else {
                continue;
            };
            for block in response.blocks() {
                if let ContentBlock::RuntimeToolCall(call) = block {
                    calls.push(call);
                }
            }
        }
    }
    calls
}

fn generated(
    call: &RuntimeToolCall,
    assigned: &HashSet<String>,
    external: &HashSet<&str>,
) -> String {
    let base = format!("call_{}", call.id().as_uuid().simple());
    let taken = |candidate: &str| assigned.contains(candidate) || external.contains(candidate);
    if !taken(&base) {
        return base;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}_{suffix}");
        if !taken(&candidate) {
            return candidate;
        }
        suffix = suffix.saturating_add(1);
    }
}
