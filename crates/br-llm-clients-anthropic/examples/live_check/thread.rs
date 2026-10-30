use std::error::Error;

use br_llm_clients_core::{ClientRequest, OutputMode, ToolSpec};
use br_llm_messages::{
    AITurn, AgentId, Author, BlockId, Conversation, ConversationId, EntryId, HumanId, HumanMessage,
    IncomingEntry, InputTurn, ModelResponse, Perspective, PlainBlock, PlainText, RuntimeToolCall,
    RuntimeToolResult, RuntimeToolResults, TextBlock, Turn, TurnEntry, TurnId, Uuid,
};

pub(crate) type Failure = Box<dyn Error>;

pub(crate) struct Thread {
    conversation: ConversationId,
    agent: AgentId,
    question: Turn,
    reply_turn: TurnId,
    entries: Vec<TurnEntry>,
}

impl Thread {
    pub(crate) fn new(question: &str) -> Result<Self, Failure> {
        let human = HumanId::from_uuid(Uuid::now_v7())?;
        let message = HumanMessage::new(EntryId::mint(), human, vec![text_block(question)?])?;
        let question = Turn::InputTurn(InputTurn::new(
            TurnId::mint(),
            vec![IncomingEntry::from(message)],
        )?);
        Ok(Self {
            conversation: ConversationId::mint(),
            agent: AgentId::from_uuid(Uuid::now_v7())?,
            question,
            reply_turn: TurnId::mint(),
            entries: Vec::new(),
        })
    }

    pub(crate) fn request(
        &self,
        system: &str,
        tools: Vec<ToolSpec>,
        output: OutputMode,
    ) -> Result<ClientRequest, Failure> {
        let mut turns = vec![self.question.clone()];
        if !self.entries.is_empty() {
            let author = Author::new("live_check")?;
            turns.push(Turn::AITurn(AITurn::from_parts(
                self.reply_turn,
                self.agent,
                author,
                self.entries.clone(),
            )?));
        }
        let conversation = Conversation::from_parts(self.conversation, turns)?;
        let projected = Perspective::of(self.agent).project(&conversation);
        Ok(ClientRequest::new(
            Some(PlainText::new(system)?),
            projected,
            self.reply_turn,
            EntryId::mint(),
            tools,
            output,
        )?)
    }

    pub(crate) fn answer_tools(
        &mut self,
        response: ModelResponse,
        results: Vec<RuntimeToolResult>,
    ) -> Result<(), Failure> {
        self.entries.push(TurnEntry::ModelResponse(response));
        self.entries
            .push(TurnEntry::RuntimeToolResults(RuntimeToolResults::new(
                EntryId::mint(),
                results,
            )?));
        Ok(())
    }
}

fn text_block(text: &str) -> Result<PlainBlock, Failure> {
    Ok(PlainBlock::Text(TextBlock::new(BlockId::mint(), text)?))
}

pub(crate) fn fake_weather(call: &RuntimeToolCall) -> Result<RuntimeToolResult, Failure> {
    let arguments = call.arguments().to_json_string()?;
    let text = format!("Weather for {arguments}: 18 degrees Celsius, clear sky, light wind.");
    Ok(RuntimeToolResult::new(
        BlockId::mint(),
        call.id(),
        call.name().clone(),
        false,
        vec![text_block(&text)?],
    )?)
}
