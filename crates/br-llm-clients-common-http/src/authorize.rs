use http::{HeaderName, HeaderValue};

use crate::authorizer::AuthorizedHeader;
use crate::header::{describe_name, is_reserved};
use crate::prepare::Prepared;
use crate::redact::Redactor;
use crate::send_error::SendError;

pub(crate) fn merge(
    prepared: &mut Prepared,
    returned: Vec<AuthorizedHeader>,
) -> Result<Redactor, SendError> {
    let redactor = Redactor::new(returned.iter().map(|header| header.value.expose()));
    let mut checked: Vec<(HeaderName, HeaderValue)> = Vec::with_capacity(returned.len());
    for header in &returned {
        let shown = describe_name(&header.name);
        let fail = |reason: &str| SendError::InvalidAuthorization {
            text: redactor.clean(&format!("header `{shown}`: {reason}")),
        };
        let name = HeaderName::from_bytes(header.name.as_bytes())
            .map_err(|_| fail("the name is not a valid header name"))?;
        if is_reserved(&name) {
            return Err(fail("the name is managed by the library"));
        }
        let mut value = HeaderValue::from_str(header.value.expose())
            .map_err(|_| fail("the value is not a valid header value"))?;
        if checked.iter().any(|(existing, _)| *existing == name) {
            return Err(fail("the name is given twice"));
        }
        value.set_sensitive(true);
        checked.push((name, value));
    }
    for (name, value) in checked {
        match prepared
            .headers
            .iter_mut()
            .find(|(existing, _)| *existing == name)
        {
            Some(slot) => slot.1 = value,
            None => prepared.headers.push((name, value)),
        }
    }
    Ok(redactor)
}
