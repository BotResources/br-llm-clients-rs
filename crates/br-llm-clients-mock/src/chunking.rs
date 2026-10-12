use crate::build_error::MockBuildError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Chunking {
    text: Option<usize>,
    thinking: Option<usize>,
    tool_arguments: Option<usize>,
    structured: Option<usize>,
}

impl Chunking {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn text(mut self, size: usize) -> Self {
        self.text = Some(size);
        self
    }

    pub fn thinking(mut self, size: usize) -> Self {
        self.thinking = Some(size);
        self
    }

    pub fn tool_arguments(mut self, size: usize) -> Self {
        self.tool_arguments = Some(size);
        self
    }

    pub fn structured(mut self, size: usize) -> Self {
        self.structured = Some(size);
        self
    }

    pub fn text_size(&self) -> Option<usize> {
        self.text
    }

    pub fn thinking_size(&self) -> Option<usize> {
        self.thinking
    }

    pub fn tool_arguments_size(&self) -> Option<usize> {
        self.tool_arguments
    }

    pub fn structured_size(&self) -> Option<usize> {
        self.structured
    }

    pub fn check(&self) -> Result<(), MockBuildError> {
        let sizes = [
            self.text,
            self.thinking,
            self.tool_arguments,
            self.structured,
        ];
        if sizes.contains(&Some(0)) {
            return Err(MockBuildError::ChunkSizeZero);
        }
        Ok(())
    }
}

pub(crate) fn pieces(text: &str, size: Option<usize>) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let Some(size) = size else {
        return vec![text];
    };
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut counted = 0;
    for (offset, _) in text.char_indices() {
        if counted == size {
            pieces.push(&text[start..offset]);
            start = offset;
            counted = 0;
        }
        counted += 1;
    }
    pieces.push(&text[start..]);
    pieces
}
