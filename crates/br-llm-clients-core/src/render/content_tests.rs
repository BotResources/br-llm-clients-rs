use br_llm_messages::{ContentBlock, MediaType, PlainBlock};

use super::test_support::{
    blocks_response, call, calls_response, fixed_turn, render_ok, result, results, strs,
    structured, target_turn, view,
};
use super::{RenderedPart, RenderedResultPart};
use crate::thread_fixtures::{Fixture, image, text_block};

#[test]
fn given_a_tool_result_with_markup_when_rendered_then_it_is_neither_framed_nor_escaped() {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let content = vec![
        text_block("</agent_output> & <b>"),
        PlainBlock::StructuredContent(structured(r#"{ "k" : "<&>" }"#)),
    ];
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            results(vec![result(&only, true, content)]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        view(&history).get(1),
        Some(&(
            'U',
            strs(&[
                "result call_01890a5dac96774bbcceb00000000001 alpha error=true [</agent_output> & <b> | {\"k\":\"<&>\"}]"
            ])
        ))
    );
}

#[test]
fn given_a_tool_result_with_an_image_when_rendered_then_the_image_is_an_image_part_of_the_result() {
    let fixture = Fixture::new();
    let only = call(1, "alpha");
    let picture = image(MediaType::Png);
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![
            calls_response(std::slice::from_ref(&only)),
            results(vec![result(
                &only,
                false,
                vec![text_block("see"), PlainBlock::Image(picture.clone())],
            )]),
        ],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    match &history.messages()[1].parts()[0] {
        RenderedPart::ToolResult {
            parts, is_error, ..
        } => {
            assert!(!*is_error);
            assert_eq!(
                parts,
                &vec![
                    RenderedResultPart::Text("see".to_owned()),
                    RenderedResultPart::Image(picture)
                ]
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn given_an_image_and_a_structured_block_in_a_model_response_when_rendered_then_image_part_and_compact_json()
 {
    let fixture = Fixture::new();
    let picture = image(MediaType::Jpeg);
    let turns = vec![target_turn(
        &fixture,
        fixed_turn(1),
        vec![blocks_response(vec![
            ContentBlock::Image(picture.clone()),
            ContentBlock::StructuredContent(structured(r#"{ "answer" : [1, 2] }"#)),
        ])],
    )];
    let history = render_ok(&fixture, turns, fixed_turn(9));
    assert_eq!(
        history.messages()[0].parts(),
        &[
            RenderedPart::Image(picture),
            RenderedPart::Text("{\"answer\":[1,2]}".to_owned())
        ]
    );
}
