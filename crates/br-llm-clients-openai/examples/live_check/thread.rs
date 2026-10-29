use br_llm_clients_core::{ClientRequest, OutputMode, ToolSpec};
use br_llm_messages::{
    AITurn, AgentId, Author, BlockId, Conversation, ConversationId, EntryId, HumanId, HumanMessage,
    IncomingEntry, InputTurn, Perspective, PlainBlock, PlainText, TextBlock, Turn, TurnEntry,
    TurnId,
};

use crate::Failure;

const AUTHOR: &str = "live-check";

pub struct Thread {
    agent: AgentId,
    question: Turn,
    reply_turn: TurnId,
    reply_entries: Vec<TurnEntry>,
}

pub fn text_block(text: &str) -> Result<PlainBlock, Failure> {
    Ok(PlainBlock::Text(TextBlock::new(BlockId::mint(), text)?))
}

impl Thread {
    pub fn new(question: &str) -> Result<Self, Failure> {
        let human = HumanId::from_uuid(TurnId::mint().as_uuid())?;
        let message = HumanMessage::new(EntryId::mint(), human, vec![text_block(question)?])?;
        let input = InputTurn::new(TurnId::mint(), vec![IncomingEntry::from(message)])?;
        Ok(Self {
            agent: AgentId::from_uuid(TurnId::mint().as_uuid())?,
            question: Turn::from(input),
            reply_turn: TurnId::mint(),
            reply_entries: Vec::new(),
        })
    }

    pub fn push(&mut self, entry: TurnEntry) {
        self.reply_entries.push(entry);
    }

    pub fn request(&self, system: &str, tools: Vec<ToolSpec>) -> Result<ClientRequest, Failure> {
        let mut turns = vec![self.question.clone()];
        if !self.reply_entries.is_empty() {
            let reply = AITurn::from_parts(
                self.reply_turn,
                self.agent,
                Author::new(AUTHOR)?,
                self.reply_entries.clone(),
            )?;
            turns.push(Turn::from(reply));
        }
        let conversation = Conversation::from_parts(ConversationId::mint(), turns)?;
        let history = Perspective::of(self.agent).project(&conversation);
        Ok(ClientRequest::new(
            Some(PlainText::new(system)?),
            history,
            self.reply_turn,
            EntryId::mint(),
            tools,
            OutputMode::Plain,
        )?)
    }
}
