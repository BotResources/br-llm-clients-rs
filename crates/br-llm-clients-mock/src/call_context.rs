use br_llm_clients_core::{ClientRequest, ClientSettings, StreamSink};

use crate::tracker::CallTracker;

pub(crate) struct CallContext<'a> {
    pub(crate) tracker: &'a CallTracker,
    pub(crate) settings: &'a ClientSettings,
    pub(crate) request: &'a ClientRequest,
    pub(crate) sink: &'a mut dyn StreamSink,
}
