use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorInfo {
    status: Option<u16>,
    code: Option<String>,
    retry_after: Option<Duration>,
    text: String,
}

impl ErrorInfo {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            status: None,
            code: None,
            retry_after: None,
            text: text.into(),
        }
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_retry_after(mut self, retry_after: Duration) -> Self {
        self.retry_after = Some(retry_after);
        self
    }

    pub fn status(&self) -> Option<u16> {
        self.status
    }

    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    pub fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}
