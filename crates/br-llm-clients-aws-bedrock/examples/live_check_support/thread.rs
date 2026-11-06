use br_llm_clients_core::{ClientRequest, OutputMode, ToolSpec};
use br_llm_messages::{
    AITurn, AgentId, Author, BlockId, Conversation, ConversationId, EntryId, HumanId, HumanMessage,
    IncomingEntry, InputTurn, ModelResponse, Perspective, PlainBlock, PlainText, RuntimeToolResult,
    RuntimeToolResults, TextBlock, Turn, TurnEntry, TurnId, Uuid,
};

use crate::fail;

pub struct Thread {
    conversation: ConversationId,
    agent: AgentId,
    human: HumanId,
    author: Author,
    earlier: Vec<Turn>,
    reply: Option<AITurn>,
    reply_turn: TurnId,
}

impl Thread {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            conversation: ConversationId::mint(),
            agent: AgentId::from_uuid(Uuid::now_v7()).map_err(fail)?,
            human: HumanId::from_uuid(Uuid::now_v7()).map_err(fail)?,
            author: Author::new("bedrock").map_err(fail)?,
            earlier: Vec::new(),
            reply: None,
            reply_turn: TurnId::mint(),
        })
    }

    pub fn say(&mut self, text: &str) -> Result<(), String> {
        if let Some(reply) = self.reply.take() {
            self.earlier.push(Turn::AITurn(reply));
        }
        let message = HumanMessage::new(EntryId::mint(), self.human, vec![plain_text(text)?])
            .map_err(fail)?;
        let input =
            InputTurn::new(TurnId::mint(), vec![IncomingEntry::from(message)]).map_err(fail)?;
        self.earlier.push(Turn::InputTurn(input));
        self.reply_turn = TurnId::mint();
        Ok(())
    }

    pub fn request(
        &self,
        system: Option<&str>,
        tools: Vec<ToolSpec>,
        output: OutputMode,
    ) -> Result<ClientRequest, String> {
        let mut turns = self.earlier.clone();
        turns.extend(self.reply.clone().map(Turn::AITurn));
        let conversation = Conversation::from_parts(self.conversation, turns).map_err(fail)?;
        let history = Perspective::of(self.agent).project(&conversation);
        let system = system.map(PlainText::new).transpose().map_err(fail)?;
        ClientRequest::new(
            system,
            history,
            self.reply_turn,
            EntryId::mint(),
            tools,
            output,
        )
        .map_err(fail)
    }

    pub fn record(&mut self, response: ModelResponse) -> Result<(), String> {
        self.push(TurnEntry::ModelResponse(response))
    }

    pub fn deliver(&mut self, results: Vec<RuntimeToolResult>) -> Result<(), String> {
        let delivery = RuntimeToolResults::new(EntryId::mint(), results).map_err(fail)?;
        self.push(TurnEntry::RuntimeToolResults(delivery))
    }

    fn push(&mut self, entry: TurnEntry) -> Result<(), String> {
        let reply = self
            .reply
            .get_or_insert_with(|| AITurn::open(self.reply_turn, self.agent, self.author.clone()));
        reply.push_entry(entry).map_err(fail)
    }
}

pub fn plain_text(text: &str) -> Result<PlainBlock, String> {
    Ok(PlainBlock::Text(
        TextBlock::new(BlockId::mint(), text).map_err(fail)?,
    ))
}
