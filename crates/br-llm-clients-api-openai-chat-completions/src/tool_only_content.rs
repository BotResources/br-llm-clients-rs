#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum ToolOnlyContent {
    Omitted,
    EmptyText,
}

impl ToolOnlyContent {
    pub(crate) fn writes_empty_text(self) -> bool {
        match self {
            ToolOnlyContent::Omitted => false,
            ToolOnlyContent::EmptyText => true,
        }
    }
}
