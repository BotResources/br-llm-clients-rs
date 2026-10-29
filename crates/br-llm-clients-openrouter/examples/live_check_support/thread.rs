use br_llm_clients_core::{ClientRequest, OutputMode, ToolSpec};
use br_llm_messages::{
    AITurn, AgentId, Author, BlockId, Conversation, ConversationId, EntryId, HumanId, HumanMessage,
    IncomingEntry, InputTurn, Perspective, PlainBlock, PlainText, TextBlock, Turn, TurnEntry,
    TurnId, Uuid,
};

use crate::Failure;

const AUTHOR: &str = "live-check";

pub struct Thread {
    agent: AgentId,
    question: Turn,
    reply_turn: TurnId,
    entries: Vec<TurnEntry>,
}

impl Thread {
    pub fn new(question: &str) -> Result<Self, Failure> {
        let human = HumanId::from_uuid(Uuid::now_v7())?;
        let text = PlainBlock::Text(TextBlock::new(BlockId::mint(), question)?);
        let message = HumanMessage::new(EntryId::mint(), human, vec![text])?;
        let input = InputTurn::new(TurnId::mint(), vec![IncomingEntry::from(message)])?;
        Ok(Self {
            agent: AgentId::from_uuid(Uuid::now_v7())?,
            question: Turn::InputTurn(input),
            reply_turn: TurnId::mint(),
            entries: Vec::new(),
        })
    }

    pub fn push(&mut self, entry: TurnEntry) {
        self.entries.push(entry);
    }

    pub fn request(
        &self,
        system: Option<&str>,
        tools: Vec<ToolSpec>,
        output: OutputMode,
    ) -> Result<ClientRequest, Failure> {
        let mut turns = vec![self.question.clone()];
        if !self.entries.is_empty() {
            let author = Author::new(AUTHOR)?;
            let reply =
                AITurn::from_parts(self.reply_turn, self.agent, author, self.entries.clone())?;
            turns.push(Turn::AITurn(reply));
        }
        let conversation = Conversation::from_parts(ConversationId::mint(), turns)?;
        let history = Perspective::of(self.agent).project(&conversation);
        let system = system.map(PlainText::new).transpose()?;
        Ok(ClientRequest::new(
            system,
            history,
            self.reply_turn,
            EntryId::mint(),
            tools,
            output,
        )?)
    }
}
