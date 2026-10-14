#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    event: Option<String>,
    data: String,
}

impl Frame {
    pub(crate) fn new(event: Option<String>, data: impl Into<String>) -> Self {
        Self {
            event: event.filter(|name| !name.is_empty()),
            data: data.into(),
        }
    }

    pub fn event(&self) -> Option<&str> {
        self.event.as_deref()
    }

    pub fn data(&self) -> &str {
        &self.data
    }

    pub fn into_parts(self) -> (Option<String>, String) {
        (self.event, self.data)
    }
}
