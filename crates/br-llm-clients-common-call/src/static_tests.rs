use std::sync::Arc;

use crate::call_spec::CallSpec;
use crate::dialect::Dialect;
use crate::payload_decoder::PayloadDecoder;
use crate::run::run;
use crate::test_dialect::ScriptedDialect;
use crate::test_fixtures::{Discard, spec_for, user_request};
use crate::test_server::closed_endpoint;

fn is_send<T: Send>(_: &T) {}

fn is_send_sync<T: Send + Sync + ?Sized>() {}

#[test]
fn given_the_public_types_when_compiled_then_the_spec_is_shared_and_the_decoder_moves() {
    is_send_sync::<CallSpec>();
    is_send_sync::<dyn Dialect>();
    is_send_sync::<crate::provider_event::ProviderEvent>();
    fn decoder_is_send(decoder: Box<dyn PayloadDecoder>) -> impl Send {
        decoder
    }
    let _ = decoder_is_send;
}

#[tokio::test]
async fn given_a_call_when_built_then_its_future_is_send() {
    let dialect = Arc::new(ScriptedDialect::default());
    let spec = spec_for(&dialect, closed_endpoint());
    let request = user_request();
    let mut sink = Discard;
    let call = run(&spec, &request, &mut sink);
    is_send(&call);
}
