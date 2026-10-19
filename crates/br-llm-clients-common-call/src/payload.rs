use br_llm_clients_common_sse::Frame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    event: Option<String>,
    data: String,
}

impl Payload {
    pub fn new(event: Option<String>, data: impl Into<String>) -> Self {
        Self {
            event,
            data: data.into(),
        }
    }

    pub fn event(&self) -> Option<&str> {
        self.event.as_deref()
    }

    pub fn data(&self) -> &str {
        &self.data
    }

    pub(crate) fn from_frame(frame: Frame) -> Self {
        let (event, data) = frame.into_parts();
        Self { event, data }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{text}")]
pub struct PayloadError {
    text: String,
}

impl PayloadError {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}
