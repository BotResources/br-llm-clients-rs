use crate::run_support::{Called, call_with};
use crate::test_server::{Wire, http};
use crate::{ChatCompletionsDialect, ClientRequest};

pub(crate) const RECORDED_ON: &str = "2026-10-19";

#[derive(Clone, Copy)]
pub(crate) struct Recording {
    pub(crate) name: &'static str,
    pub(crate) head: &'static str,
    pub(crate) body: &'static str,
    pub(crate) recorded_on: &'static str,
}

const fn recording(name: &'static str, head: &'static str, body: &'static str) -> Recording {
    Recording {
        name,
        head,
        body,
        recorded_on: RECORDED_ON,
    }
}

pub(crate) const TEXT: Recording = recording(
    "chat-text",
    include_str!("recordings/chat-text.hdr"),
    include_str!("recordings/chat-text.sse"),
);
pub(crate) const TOOL: Recording = recording(
    "chat-tool",
    include_str!("recordings/chat-tool.hdr"),
    include_str!("recordings/chat-tool.sse"),
);
pub(crate) const LENGTH: Recording = recording(
    "chat-length",
    include_str!("recordings/chat-length.hdr"),
    include_str!("recordings/chat-length.sse"),
);
pub(crate) const REASONING: Recording = recording(
    "chat-reasoning",
    include_str!("recordings/chat-reasoning.hdr"),
    include_str!("recordings/chat-reasoning.sse"),
);
pub(crate) const REFUSAL: Recording = recording(
    "chat-refusal",
    include_str!("recordings/chat-refusal.hdr"),
    include_str!("recordings/chat-refusal.sse"),
);
pub(crate) const BAD_KEY: Recording = recording(
    "chat-badkey",
    include_str!("recordings/chat-badkey.hdr"),
    include_str!("recordings/chat-badkey.json"),
);
pub(crate) const BAD_MODEL: Recording = recording(
    "chat-badmodel",
    include_str!("recordings/chat-badmodel.hdr"),
    include_str!("recordings/chat-badmodel.json"),
);
pub(crate) const BAD_BODY: Recording = recording(
    "chat-badbody",
    include_str!("recordings/chat-badbody.hdr"),
    include_str!("recordings/chat-badbody.json"),
);

pub(crate) const RECORDINGS: [Recording; 8] = [
    TEXT, TOOL, LENGTH, REASONING, REFUSAL, BAD_KEY, BAD_MODEL, BAD_BODY,
];

impl Recording {
    pub(crate) fn status(&self) -> u16 {
        self.head
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("a status line")
    }

    pub(crate) fn headers(&self) -> Vec<(&'static str, &'static str)> {
        self.head
            .split("\r\n")
            .skip(1)
            .take_while(|line| !line.is_empty())
            .filter_map(|line| line.split_once(": "))
            .collect()
    }

    pub(crate) fn header(&self, name: &str) -> Option<&'static str> {
        self.headers()
            .into_iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
            .map(|(_, value)| value)
    }

    pub(crate) fn request_id(&self) -> &'static str {
        self.header("x-request-id").expect("a request id")
    }

    pub(crate) fn frames(&self) -> Vec<String> {
        self.body
            .split("\n\n")
            .filter(|frame| !frame.is_empty())
            .map(|frame| {
                frame
                    .strip_prefix("data: ")
                    .expect("a data frame")
                    .to_owned()
            })
            .collect()
    }

    pub(crate) fn wire(&self) -> Wire {
        let status = format!("{} Recorded", self.status());
        let headers: Vec<(&str, &str)> = self
            .headers()
            .into_iter()
            .filter(|(name, _)| !name.eq_ignore_ascii_case("content-length"))
            .collect();
        http(&status, &headers, self.body.as_bytes())
    }
}

pub(crate) async fn call_recorded(
    recording: Recording,
    dialect: ChatCompletionsDialect,
    request: &ClientRequest,
) -> Called {
    call_with(dialect, request, move |_| recording.wire()).await
}
