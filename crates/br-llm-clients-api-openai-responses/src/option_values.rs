#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Priority,
    Scale,
    Fast,
    Ultrafast,
}

impl ServiceTier {
    pub(crate) fn wire(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Default => "default",
            Self::Flex => "flex",
            Self::Priority => "priority",
            Self::Scale => "scale",
            Self::Fast => "fast",
            Self::Ultrafast => "ultrafast",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PromptCacheMode {
    Implicit,
    Explicit,
}

impl PromptCacheMode {
    pub(crate) fn wire(self) -> &'static str {
        match self {
            Self::Implicit => "implicit",
            Self::Explicit => "explicit",
        }
    }
}
