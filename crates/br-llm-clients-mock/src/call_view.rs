use br_llm_clients_core::{ClientRequest, ClientSettings};

#[derive(Debug, Clone, Copy)]
pub struct MatchContext<'a> {
    request: &'a ClientRequest,
    settings: &'a ClientSettings,
    sequence: usize,
}

impl<'a> MatchContext<'a> {
    pub(crate) fn new(
        request: &'a ClientRequest,
        settings: &'a ClientSettings,
        sequence: usize,
    ) -> Self {
        Self {
            request,
            settings,
            sequence,
        }
    }

    pub fn request(&self) -> &'a ClientRequest {
        self.request
    }

    pub fn settings(&self) -> &'a ClientSettings {
        self.settings
    }

    pub fn sequence(&self) -> usize {
        self.sequence
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReplyContext<'a> {
    call: MatchContext<'a>,
    claimed_before: usize,
}

impl<'a> ReplyContext<'a> {
    pub(crate) fn new(call: MatchContext<'a>, claimed_before: usize) -> Self {
        Self {
            call,
            claimed_before,
        }
    }

    pub fn request(&self) -> &'a ClientRequest {
        self.call.request()
    }

    pub fn settings(&self) -> &'a ClientSettings {
        self.call.settings()
    }

    pub fn sequence(&self) -> usize {
        self.call.sequence()
    }

    pub fn claimed_before(&self) -> usize {
        self.claimed_before
    }
}
