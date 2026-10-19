pub(crate) const RECORDED_ON: &str = "2026-10-19";

#[derive(Debug, Clone, Copy)]
pub(crate) struct Recording {
    pub(crate) name: &'static str,
    pub(crate) file: &'static str,
    pub(crate) status: u16,
    pub(crate) status_line: &'static str,
    pub(crate) content_type: &'static str,
    pub(crate) request_id: &'static str,
    pub(crate) body: &'static str,
    pub(crate) recorded_on: &'static str,
}

pub(crate) const TEXT: Recording = Recording {
    name: "TEXT",
    file: "text.sse",
    status: 200,
    status_line: "200 OK",
    content_type: "text/event-stream; charset=utf-8",
    request_id: "req_ee17b7bdd562435083cfcd0cdeb5b886",
    body: include_str!("recordings/text.sse"),
    recorded_on: RECORDED_ON,
};

pub(crate) const REASONING_TOOL: Recording = Recording {
    name: "REASONING_TOOL",
    file: "reasoning_tool.sse",
    status: 200,
    status_line: "200 OK",
    content_type: "text/event-stream; charset=utf-8",
    request_id: "req_a1dfc4bb67564b349a13c7298de375c8",
    body: include_str!("recordings/reasoning_tool.sse"),
    recorded_on: RECORDED_ON,
};

pub(crate) const INCOMPLETE: Recording = Recording {
    name: "INCOMPLETE",
    file: "incomplete.sse",
    status: 200,
    status_line: "200 OK",
    content_type: "text/event-stream; charset=utf-8",
    request_id: "req_0b6520e6dc6f4acbb39d091aa37c7eef",
    body: include_str!("recordings/incomplete.sse"),
    recorded_on: RECORDED_ON,
};

pub(crate) const REFUSAL: Recording = Recording {
    name: "REFUSAL",
    file: "refusal.sse",
    status: 200,
    status_line: "200 OK",
    content_type: "text/event-stream; charset=utf-8",
    request_id: "req_af92444b1de641adbe51fc9b91bfcb6f",
    body: include_str!("recordings/refusal.sse"),
    recorded_on: RECORDED_ON,
};

pub(crate) const BAD_KEY: Recording = Recording {
    name: "BAD_KEY",
    file: "bad_key.json",
    status: 401,
    status_line: "401 Unauthorized",
    content_type: "text/plain",
    request_id: "req_c2a100e0e1a14fe5b0dce2af2fdbd918",
    body: include_str!("recordings/bad_key.json"),
    recorded_on: RECORDED_ON,
};

pub(crate) const BAD_MODEL: Recording = Recording {
    name: "BAD_MODEL",
    file: "bad_model.json",
    status: 404,
    status_line: "404 Not Found",
    content_type: "application/json",
    request_id: "req_c7a0567a45394989ad4fbf19a4c5a067",
    body: include_str!("recordings/bad_model.json"),
    recorded_on: RECORDED_ON,
};

pub(crate) const BAD_BODY: Recording = Recording {
    name: "BAD_BODY",
    file: "bad_body.json",
    status: 400,
    status_line: "400 Bad Request",
    content_type: "application/json",
    request_id: "req_5fd582a40dc34e379ea8919f338c8cb7",
    body: include_str!("recordings/bad_body.json"),
    recorded_on: RECORDED_ON,
};

pub(crate) const RECORDINGS: [Recording; 7] = [
    TEXT,
    REASONING_TOOL,
    INCOMPLETE,
    REFUSAL,
    BAD_KEY,
    BAD_MODEL,
    BAD_BODY,
];
