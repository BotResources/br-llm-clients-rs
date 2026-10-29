use std::fmt;

use crate::service_account_key::ServiceAccountKey;

#[derive(Clone)]
#[non_exhaustive]
pub enum VertexCredential {
    ServiceAccount(ServiceAccountKey),
}

impl fmt::Debug for VertexCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ServiceAccount(_) => f.write_str("ServiceAccount"),
        }
    }
}
