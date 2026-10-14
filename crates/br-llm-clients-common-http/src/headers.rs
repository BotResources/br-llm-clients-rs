use http::{HeaderMap, HeaderName};

#[derive(Clone, Default)]
pub struct Headers {
    map: HeaderMap,
}

impl Headers {
    pub(crate) fn new(map: HeaderMap) -> Self {
        Self { map }
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.get_all(name).next()
    }

    pub fn get_all(&self, name: &str) -> impl Iterator<Item = &[u8]> {
        HeaderName::from_bytes(name.as_bytes())
            .ok()
            .into_iter()
            .flat_map(|parsed| self.map.get_all(parsed).iter())
            .map(http::HeaderValue::as_bytes)
    }

    pub fn get_text(&self, name: &str) -> Option<&str> {
        self.get(name)
            .and_then(|value| std::str::from_utf8(value).ok())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.map
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_bytes()))
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.map.keys().map(HeaderName::as_str)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl std::fmt::Debug for Headers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.names()).finish()
    }
}
