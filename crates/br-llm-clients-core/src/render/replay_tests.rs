use br_llm_messages::{
    Citation, CitationId, CitationSource, ClosureReason, ContentBlock, ProviderToolCall,
    ProviderToolExecutionRecord, ProviderToolResult, RedactedThinkingBlock, TextBlock,
    ThinkingBlock,
};

use super::RenderedPart;
use super::test_support::{
    blocks_response, call, calls_response, closure, fixed_turn, human_turn_of, render_ok, replay,
    result, results, strs, target_turn, text, text_with_item_ref, text_with_replay, view,
};
use crate::thread_fixtures::{Fixture, block_id, text_block};

fn thinking() -> ContentBlock {
    ContentBlock::Thinking(
        ThinkingBlock::new(block_id(), "pondering")
            .unwrap()
            .with_replay(replay("sig")),
    )
}

fn redacted() -> ContentBlock {
    ContentBlock::RedactedThinking(RedactedThinkingBlock::new(block_id(), replay("hidden")))
}

fn provider_blocks() -> Vec<ContentBlock> {
    let provider_call = ProviderToolCall::new(block_id());
    let provider_result =
        ProviderToolResult::new(block_id(), provider_call.id(), vec![text_block("found")]).unwrap();
    vec![
        ContentBlock::ProviderToolCall(provider_call),
        ContentBlock::ProviderToolResult(provider_result),
        ContentBlock::ProviderToolExecutionRecord(ProviderToolExecutionRecord::new(block_id())),
    ]
}

fn mixed() -> Vec<ContentBlock> {
    let mut blocks = vec![thinking(), redacted()];
    blocks.extend(provider_blocks());
    blocks.push(text("visible"));
    blocks
}

#[test]
fn given_thinking_and_provider_blocks_in_the_reply_turn_when_rendered_then_they_are_block_parts_as_they_came()
 {
    let fixture = Fixture::new();
    let blocks = mixed();
    let turns = vec![
        human_turn_of("go"),
        target_turn(
            &fixture,
            fixed_turn(1),
            vec![blocks_response(blocks.clone())],
        ),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(1));
    assert_eq!(
        view(&history).get(1),
        Some(&(
            'A',
            strs(&[
                "block thinking",
                "block redacted_thinking",
                "block provider_tool_call",
                "block provider_tool_result",
                "block provider_tool_execution_record",
                "visible"
            ])
        ))
    );
    let parts = history.messages()[1].parts();
    assert_eq!(parts[0], RenderedPart::Block(blocks[0].clone()));
    assert_eq!(parts[1], RenderedPart::Block(blocks[1].clone()));
    assert_eq!(parts[4], RenderedPart::Block(blocks[4].clone()));
}

#[test]
fn given_thinking_and_provider_blocks_in_another_turn_when_rendered_then_only_the_text_remains() {
    let fixture = Fixture::new();
    let turns = vec![
        human_turn_of("go"),
        target_turn(&fixture, fixed_turn(1), vec![blocks_response(mixed())]),
    ];
    let history = render_ok(&fixture, turns, fixed_turn(2));
    assert_eq!(view(&history).get(1), Some(&('A', strs(&["visible"]))));
}

#[test]
fn given_text_with_replay_or_item_reference_in_the_reply_turn_when_rendered_then_it_is_a_block_part()
 {
    let fixture = Fixture::new();
    let with_replay = text_with_replay("sealed text");
    let with_item = text_with_item_ref("referenced text");
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![blocks_response(vec![
            with_replay.clone(),
            with_item.clone(),
            text("plain text"),
        ])],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(1));
    let parts = history.messages()[0].parts();
    assert_eq!(parts[0], RenderedPart::Block(with_replay));
    assert_eq!(parts[1], RenderedPart::Block(with_item));
    assert_eq!(parts[2], RenderedPart::Text("plain text".to_owned()));
}

#[test]
fn given_text_with_replay_or_item_reference_in_another_turn_when_rendered_then_it_stays_a_text_part()
 {
    let fixture = Fixture::new();
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![blocks_response(vec![
            text_with_replay("sealed text"),
            text_with_item_ref("referenced text"),
        ])],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(2));
    assert_eq!(
        view(&history),
        vec![('A', strs(&["sealed text", "referenced text"]))]
    );
    assert_eq!(
        history.messages()[0].parts()[0],
        RenderedPart::Text("sealed text".to_owned())
    );
}

#[test]
fn given_a_text_block_with_citations_when_rendered_then_only_the_text_is_rendered() {
    let fixture = Fixture::new();
    let citation = Citation::new(
        CitationId::parse("01890a5d-ac96-774b-bcce-c00000000001").unwrap(),
        CitationSource::new("https://example.test").unwrap(),
    );
    let cited = TextBlock::new(block_id(), "cited text")
        .unwrap()
        .with_citations(vec![citation])
        .unwrap();
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![blocks_response(vec![ContentBlock::Text(cited)])],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(1));
    assert_eq!(
        history.messages()[0].parts(),
        &[RenderedPart::Text("cited text".to_owned())]
    );
}

fn rich_call() -> br_llm_messages::RuntimeToolCall {
    call(1, "alpha")
        .with_replay(replay("call-seal"))
        .with_item_ref(br_llm_messages::ItemRef::new("fc_1").unwrap())
        .with_external_id(br_llm_messages::ExternalId::new("toolu_77").unwrap())
}

fn call_turn(
    fixture: &Fixture,
    turn: u32,
    with_call: &br_llm_messages::RuntimeToolCall,
) -> br_llm_messages::Turn {
    target_turn(
        fixture,
        fixed_turn(turn),
        vec![
            calls_response(std::slice::from_ref(with_call)),
            results(vec![result(with_call, false, vec![text_block("ok")])]),
            closure(ClosureReason::completed()),
        ],
    )
}

#[test]
fn given_a_tool_call_of_the_reply_turn_when_rendered_then_it_carries_replay_and_item_reference() {
    let fixture = Fixture::new();
    let rich = rich_call();
    let history = render_ok(&fixture, vec![call_turn(&fixture, 1, &rich)], fixed_turn(1));
    assert_eq!(
        history.messages()[0].parts(),
        &[RenderedPart::ToolCall {
            call: rich,
            wire_id: "toolu_77".to_owned()
        }]
    );
}

#[test]
fn given_a_tool_call_of_another_turn_when_rendered_then_it_carries_no_replay_and_no_item_reference()
{
    let fixture = Fixture::new();
    let rich = rich_call();
    let history = render_ok(&fixture, vec![call_turn(&fixture, 1, &rich)], fixed_turn(2));
    let expected =
        call(1, "alpha").with_external_id(br_llm_messages::ExternalId::new("toolu_77").unwrap());
    assert_eq!(
        history.messages()[0].parts(),
        &[RenderedPart::ToolCall {
            call: expected,
            wire_id: "toolu_77".to_owned()
        }]
    );
    match &history.messages()[0].parts()[0] {
        RenderedPart::ToolCall { call, .. } => {
            assert!(call.replay().is_none());
            assert!(call.item_ref().is_none());
        }
        other => panic!("{other:?}"),
    }
}
