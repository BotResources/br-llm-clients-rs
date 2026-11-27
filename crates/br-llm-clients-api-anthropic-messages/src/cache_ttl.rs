#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CacheTtl {
    #[default]
    FiveMinutes,
    OneHour,
}

impl CacheTtl {
    pub fn as_str(&self) -> &'static str {
        match self {
            CacheTtl::FiveMinutes => "5m",
            CacheTtl::OneHour => "1h",
        }
    }

    pub fn parse(text: &str) -> Option<CacheTtl> {
        [CacheTtl::FiveMinutes, CacheTtl::OneHour]
            .into_iter()
            .find(|ttl| ttl.as_str() == text)
    }
}
