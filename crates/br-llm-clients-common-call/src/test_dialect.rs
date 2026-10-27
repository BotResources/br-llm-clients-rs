use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use br_llm_clients_common_http::Headers;
use br_llm_clients_core::{ClientError, ClientRequest, OutputMode, RenderedHistory};

use crate::dialect::Dialect;
use crate::encoded_request::EncodedRequest;
use crate::error_info::ErrorInfo;
use crate::payload_decoder::PayloadDecoder;
use crate::test_decoder::{DecoderLog, EndScript, ScriptDecoder};

pub(crate) type Refine = Box<dyn Fn(&ErrorInfo) -> Option<ClientError> + Send + Sync>;
pub(crate) type Answer = Box<dyn Fn(&Headers) -> Option<String> + Send + Sync>;

pub(crate) struct ScriptedDialect {
    pub(crate) body: Vec<u8>,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) encode_error: Option<ClientError>,
    pub(crate) refine: Refine,
    pub(crate) request_id_header: &'static str,
    pub(crate) answered: Answer,
    pub(crate) encode_calls: AtomicUsize,
    pub(crate) seen_bodies: Mutex<Vec<String>>,
    pub(crate) seen_infos: Mutex<Vec<ErrorInfo>>,
    pub(crate) log: Arc<DecoderLog>,
    pub(crate) seen_modes: Mutex<Vec<OutputMode>>,
    pub(crate) seen_histories: Mutex<Vec<(ClientRequest, RenderedHistory)>>,
    pub(crate) end: EndScript,
}

impl Default for ScriptedDialect {
    fn default() -> Self {
        Self {
            body: b"{\"ask\":1}".to_vec(),
            headers: vec![("content-type".to_owned(), "application/json".to_owned())],
            encode_error: None,
            refine: Box::new(|_| None),
            request_id_header: "x-request-id",
            answered: Box::new(|_| None),
            encode_calls: AtomicUsize::new(0),
            seen_bodies: Mutex::new(Vec::new()),
            seen_infos: Mutex::new(Vec::new()),
            log: Arc::new(DecoderLog::default()),
            seen_modes: Mutex::new(Vec::new()),
            seen_histories: Mutex::new(Vec::new()),
            end: EndScript::Events(Vec::new()),
        }
    }
}

impl ScriptedDialect {
    pub(crate) fn encode_calls(&self) -> usize {
        self.encode_calls.load(Ordering::SeqCst)
    }

    pub(crate) fn seen_bodies(&self) -> Vec<String> {
        self.seen_bodies.lock().expect("seen").clone()
    }

    pub(crate) fn seen_modes(&self) -> Vec<OutputMode> {
        self.seen_modes.lock().expect("seen").clone()
    }

    pub(crate) fn seen_histories(&self) -> Vec<(ClientRequest, RenderedHistory)> {
        self.seen_histories.lock().expect("seen").clone()
    }

    pub(crate) fn seen_infos(&self) -> Vec<ErrorInfo> {
        self.seen_infos.lock().expect("seen").clone()
    }
}

impl Dialect for ScriptedDialect {
    fn encode(
        &self,
        request: &ClientRequest,
        history: &RenderedHistory,
    ) -> Result<EncodedRequest, ClientError> {
        self.encode_calls.fetch_add(1, Ordering::SeqCst);
        self.seen_histories
            .lock()
            .expect("seen")
            .push((request.clone(), history.clone()));
        if let Some(error) = &self.encode_error {
            return Err(error.clone());
        }
        let encoded = self
            .headers
            .iter()
            .fold(EncodedRequest::new(self.body.clone()), |encoded, (n, v)| {
                encoded.with_header(n, v)
            });
        Ok(encoded)
    }

    fn decoder(&self, output: &OutputMode) -> Box<dyn PayloadDecoder> {
        self.log.note_created();
        self.seen_modes.lock().expect("seen").push(output.clone());
        Box::new(ScriptDecoder {
            log: Arc::clone(&self.log),
            end: self.end.clone(),
        })
    }

    fn error_code(&self, body: &str) -> Option<String> {
        self.seen_bodies.lock().expect("seen").push(body.to_owned());
        let (_, after) = body.split_once("\"code\":\"")?;
        let (code, _) = after.split_once('"')?;
        Some(code.to_owned())
    }

    fn refine_error(&self, info: &ErrorInfo) -> Option<ClientError> {
        self.seen_infos.lock().expect("seen").push(info.clone());
        (self.refine)(info)
    }

    fn request_id(&self, headers: &Headers) -> Option<String> {
        headers.get_text(self.request_id_header).map(str::to_owned)
    }

    fn answered_model(&self, headers: &Headers) -> Option<String> {
        (self.answered)(headers)
    }
}
