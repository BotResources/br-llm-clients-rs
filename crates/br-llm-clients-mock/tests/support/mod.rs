use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread::Thread;

use br_llm_clients_core::{
    CacheRequirement, Capabilities, ClientRequest, ClientSettings, IdSource, MaxOutputTokens,
    OutputMode, ReasoningLevel,
};
use br_llm_clients_mock::{CapabilitiesSpec, SequentialIdSource};
use br_llm_messages::{
    AgentId, Conversation, ConversationId, EntryId, HumanId, HumanMessage, InputTurn, ModelName,
    Perspective, PlainBlock, TextBlock, Turn, TurnId,
};

struct Unparker(Thread);

impl Wake for Unparker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Unparker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

pub fn everything() -> Capabilities {
    Capabilities::new(CapabilitiesSpec::everything()).unwrap()
}

pub fn settings() -> ClientSettings {
    ClientSettings::new(
        ModelName::new("test-model").unwrap(),
        ReasoningLevel::Medium,
        CacheRequirement::NotRequired,
        MaxOutputTokens::new(512).unwrap(),
    )
}

pub fn request() -> ClientRequest {
    let ids = SequentialIdSource::starting_at(1);
    let agent = AgentId::from_uuid(ids.next_block_id().as_uuid()).unwrap();
    let human = HumanId::from_uuid(ids.next_block_id().as_uuid()).unwrap();
    let block = TextBlock::new(ids.next_block_id(), "hello").unwrap();
    let message = HumanMessage::new(
        EntryId::from_uuid(ids.next_block_id().as_uuid()).unwrap(),
        human,
        vec![PlainBlock::Text(block)],
    )
    .unwrap();
    let turn = InputTurn::new(
        TurnId::from_uuid(ids.next_block_id().as_uuid()).unwrap(),
        vec![message.into()],
    )
    .unwrap();
    let conversation = Conversation::from_parts(
        ConversationId::from_uuid(ids.next_block_id().as_uuid()).unwrap(),
        vec![Turn::InputTurn(turn)],
    )
    .unwrap();
    ClientRequest::new(
        None,
        Perspective::of(agent).project(&conversation),
        ids.next_turn_id(),
        ids.next_entry_id(),
        vec![],
        OutputMode::Plain,
    )
    .unwrap()
}
