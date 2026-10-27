#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects
    )
)]

mod block_deltas;
mod build_error;
mod call;
mod call_context;
mod call_log;
mod call_view;
mod chunking;
mod claim;
mod codes;
mod derive_deltas;
mod fold;
mod fold_error;
mod gate;
mod gate_pass;
mod json_input;
mod log;
mod log_disabled;
mod mock_builder;
mod mock_client;
mod mock_waits;
mod outcomes;
mod plan;
mod recording_sink;
mod renew;
mod renew_calls;
mod reply;
mod reply_builder;
mod reply_check;
mod reply_source;
mod report_values;
mod response_check;
mod response_input;
mod rule;
mod script;
mod sequential_id_source;
mod serve;
mod shared;
mod sketch;
mod sketch_build;
mod state_counts;
mod strictness;
mod sync;
mod tracker;
mod usage_rules;

pub use br_llm_clients_core::{Accepted, Capabilities, CapabilitiesError, CapabilitiesSpec};
pub use build_error::MockBuildError;
pub use call_log::{CallState, LoggedCall, Served, StateKind};
pub use call_view::{MatchContext, ReplyContext};
pub use chunking::Chunking;
pub use codes::{CODE_IDENTITY_COLLISION, CODE_INVALID_REPLY, CODE_SCRIPT_EXHAUSTED};
pub use derive_deltas::{DerivedDeltas, derive_deltas};
pub use fold::{PartialFold, Stop, fold_deltas, fold_partial};
pub use fold_error::FoldError;
pub use gate::{Gate, WaitForWaiting};
pub use json_input::JsonInput;
pub use log_disabled::LogDisabled;
pub use mock_builder::MockBuilder;
pub use mock_client::MockClient;
pub use mock_waits::{WaitForCalls, WaitForState};
pub use recording_sink::{RecordingSink, WaitForDeltas};
pub use reply::Reply;
pub use reply_builder::{
    BrokenStreamKind, CutKind, FailureKind, HangKind, NamesUsage, ReplyBuilder, ReplyKind,
    Reported, SuccessKind,
};
pub use reply_source::ReplySource;
pub use response_input::ResponseInput;
pub use rule::Rule;
pub use sequential_id_source::{DEFAULT_ID_START, SequentialIdSource};
pub use sketch::Sketch;
