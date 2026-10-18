#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedRequest {
    body: Vec<u8>,
    headers: Vec<(String, String)>,
}

impl EncodedRequest {
    pub fn new(body: impl Into<Vec<u8>>) -> Self {
        Self {
            body: body.into(),
            headers: Vec::new(),
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }
}
