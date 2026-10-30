use br_llm_clients_core::{Client, ClientRequest, OutputMode};
use br_llm_clients_openai::OpenAiClient;
use br_llm_messages::{
    AITurn, AgentId, Author, ClosureReason, Conversation, ConversationId, EntryId, HumanId,
    HumanMessage, IncomingEntry, InputTurn, ModelResponse, Perspective, PlainBlock, PlainText,
    Turn, TurnClosure, TurnEntry, TurnId,
};

use crate::Failure;
use crate::sink::QuietSink;

const AUTHOR: &str = "live-check";

pub struct Dialogue {
    human: HumanId,
    agent: AgentId,
    turns: Vec<Turn>,
}

impl Dialogue {
    pub fn new() -> Result<Self, Failure> {
        Ok(Self {
            human: HumanId::from_uuid(TurnId::mint().as_uuid())?,
            agent: AgentId::from_uuid(TurnId::mint().as_uuid())?,
            turns: Vec::new(),
        })
    }

    pub async fn say(
        &mut self,
        client: &OpenAiClient,
        system: &str,
        blocks: Vec<PlainBlock>,
    ) -> Result<Option<ModelResponse>, Failure> {
        self.ask(blocks)?;
        let reply = TurnId::mint();
        let request = self.request(system, reply)?;
        let outcome = client.call(&request, &mut QuietSink).await;
        match outcome.into_parts().1 {
            Ok(response) => {
                self.answer(reply, response.clone())?;
                Ok(Some(response))
            }
            Err(error) => {
                println!("  error: {error}");
                Ok(None)
            }
        }
    }

    fn ask(&mut self, blocks: Vec<PlainBlock>) -> Result<(), Failure> {
        let message = HumanMessage::new(EntryId::mint(), self.human, blocks)?;
        let input = InputTurn::new(TurnId::mint(), vec![IncomingEntry::from(message)])?;
        self.turns.push(Turn::from(input));
        Ok(())
    }

    fn request(&self, system: &str, reply: TurnId) -> Result<ClientRequest, Failure> {
        let conversation = Conversation::from_parts(ConversationId::mint(), self.turns.clone())?;
        let history = Perspective::of(self.agent).project(&conversation);
        Ok(ClientRequest::new(
            Some(PlainText::new(system)?),
            history,
            reply,
            EntryId::mint(),
            Vec::new(),
            OutputMode::Plain,
        )?)
    }

    fn answer(&mut self, reply: TurnId, response: ModelResponse) -> Result<(), Failure> {
        let closure = TurnClosure::new(EntryId::mint(), ClosureReason::completed());
        let turn = AITurn::from_parts(
            reply,
            self.agent,
            Author::new(AUTHOR)?,
            vec![
                TurnEntry::ModelResponse(response),
                TurnEntry::TurnClosure(closure),
            ],
        )?;
        self.turns.push(Turn::from(turn));
        Ok(())
    }
}
