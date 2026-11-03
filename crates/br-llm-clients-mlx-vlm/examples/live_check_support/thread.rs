use std::mem;

use br_llm_clients_core::{ClientRequest, OutputMode, ToolSpec};
use br_llm_messages::{
    AITurn, AgentId, Author, BlockId, Conversation, ConversationId, EntryId, HumanId, HumanMessage,
    IncomingEntry, InputTurn, Perspective, PlainBlock, PlainText, TextBlock, Turn, TurnEntry,
    TurnId, Uuid,
};

use crate::Failure;

const AUTHOR: &str = "live-check";

pub struct Thread {
    conversation: ConversationId,
    human: HumanId,
    agent: AgentId,
    turns: Vec<Turn>,
    reply_turn: TurnId,
    entries: Vec<TurnEntry>,
}

fn input(human: HumanId, question: &str) -> Result<Turn, Failure> {
    let text = PlainBlock::Text(TextBlock::new(BlockId::mint(), question)?);
    let message = HumanMessage::new(EntryId::mint(), human, vec![text])?;
    let input = InputTurn::new(TurnId::mint(), vec![IncomingEntry::from(message)])?;
    Ok(Turn::InputTurn(input))
}

impl Thread {
    pub fn new(question: &str) -> Result<Self, Failure> {
        let human = HumanId::from_uuid(Uuid::now_v7())?;
        Ok(Self {
            conversation: ConversationId::mint(),
            human,
            agent: AgentId::from_uuid(Uuid::now_v7())?,
            turns: vec![input(human, question)?],
            reply_turn: TurnId::mint(),
            entries: Vec::new(),
        })
    }

    pub fn push(&mut self, entry: TurnEntry) {
        self.entries.push(entry);
    }

    pub fn ask(&mut self, question: &str) -> Result<(), Failure> {
        if !self.entries.is_empty() {
            let entries = mem::take(&mut self.entries);
            self.turns.push(self.reply(entries)?);
        }
        self.turns.push(input(self.human, question)?);
        self.reply_turn = TurnId::mint();
        Ok(())
    }

    fn reply(&self, entries: Vec<TurnEntry>) -> Result<Turn, Failure> {
        let author = Author::new(AUTHOR)?;
        let turn = AITurn::from_parts(self.reply_turn, self.agent, author, entries)?;
        Ok(Turn::AITurn(turn))
    }

    pub fn request(
        &self,
        system: Option<&str>,
        tools: Vec<ToolSpec>,
        output: OutputMode,
    ) -> Result<ClientRequest, Failure> {
        let mut turns = self.turns.clone();
        if !self.entries.is_empty() {
            turns.push(self.reply(self.entries.clone())?);
        }
        let conversation = Conversation::from_parts(self.conversation, turns)?;
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
