use crate::payload::{Payload, PayloadError};
use crate::provider_event::ProviderEvent;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum End {
    Events(Vec<ProviderEvent>),
    MarkerMissing,
}

pub trait PayloadDecoder: Send {
    fn payload(&mut self, payload: Payload) -> Result<Vec<ProviderEvent>, PayloadError>;

    fn end(&mut self) -> Result<End, PayloadError>;
}
