#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AzureOpenAiVersion {
    V1,
    Dated(String),
}
