#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Accepted<T> {
    Any,
    Only(Vec<T>),
}

impl<T: PartialEq> Accepted<T> {
    pub(crate) fn accepts(&self, value: &T) -> bool {
        match self {
            Accepted::Any => true,
            Accepted::Only(values) => values.contains(value),
        }
    }

    pub(crate) fn is_empty_set(&self) -> bool {
        matches!(self, Accepted::Only(values) if values.is_empty())
    }
}
