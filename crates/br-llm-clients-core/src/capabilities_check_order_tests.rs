use br_llm_messages::ImageMime;

use crate::capabilities_test_support::{
    MODEL_NAME, image_request, one_tool, plain_request, request_of, settings, shape, user_images,
    user_text,
};
use crate::{
    Accepted, CacheRequirement, Capabilities, CapabilitiesSpec, ClientError, ClientRequest,
    ClientSettings, OutputMode, ReasoningLevel,
};

const STEPS: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
const LIMIT: u32 = 100;

struct Scenario {
    spec: CapabilitiesSpec,
    level: ReasoningLevel,
    cache: CacheRequirement,
    tokens: u32,
    tools: bool,
    structured: bool,
    image: Option<ImageMime>,
}

impl Scenario {
    fn passing() -> Self {
        Self {
            spec: CapabilitiesSpec {
                reasoning_levels: Accepted::Only(vec![ReasoningLevel::Off]),
                max_output_tokens: CapabilitiesSpec::max_output_tokens_of(
                    std::num::NonZeroU32::new(LIMIT).unwrap(),
                ),
                ..CapabilitiesSpec::everything()
            },
            level: ReasoningLevel::Off,
            cache: CacheRequirement::NotRequired,
            tokens: LIMIT,
            tools: false,
            structured: false,
            image: None,
        }
    }

    fn breaking(steps: &[u8]) -> Self {
        let mut scenario = Self::passing();
        for step in STEPS {
            if steps.contains(&step) {
                scenario.break_step(step);
            }
        }
        scenario
    }

    fn break_step(&mut self, step: u8) {
        match step {
            1 => self.level = ReasoningLevel::High,
            2 => {
                self.spec.caching = false;
                self.cache = CacheRequirement::Required;
            }
            3 => self.tokens = LIMIT + 1,
            4 => {
                self.spec.tools = false;
                self.tools = true;
            }
            5 => {
                self.spec.structured_output = false;
                self.structured = true;
            }
            6 => {
                self.spec.structured_output_with_tools = false;
                self.structured = true;
                self.tools = true;
            }
            7 => {
                self.spec.images = false;
                self.image = Some(ImageMime::Png);
            }
            8 => {
                self.spec.image_media_types = Accepted::Only(vec![ImageMime::Png]);
                self.image = Some(ImageMime::Gif);
            }
            _ => unreachable!("eight steps"),
        }
    }

    fn settings(&self) -> ClientSettings {
        settings(self.level, self.cache, self.tokens)
    }

    fn request(&self) -> ClientRequest {
        let output = if self.structured {
            shape()
        } else {
            OutputMode::Plain
        };
        let tools = if self.tools { one_tool() } else { vec![] };
        let messages = match self.image {
            Some(mime) => vec![user_text("see"), user_images(&[mime])],
            None => vec![user_text("hello")],
        };
        request_of(messages, tools, output)
    }

    fn outcome(&self) -> Result<(), ClientError> {
        Capabilities::normalized(self.spec.clone()).check(&self.settings(), &self.request())
    }
}

fn refusal_of_step(step: u8) -> ClientError {
    match step {
        1 => ClientError::ReasoningNotSupported {
            level: ReasoningLevel::High,
            reason: format!(
                "the declared capabilities of model {MODEL_NAME} do not accept this level"
            ),
        },
        2 => ClientError::CacheNotSupported,
        3 => ClientError::MaxOutputTokensTooLarge {
            limit: Some(u64::from(LIMIT)),
        },
        4 => ClientError::ToolsNotSupported,
        5 => ClientError::StructuredOutputNotSupported,
        6 => ClientError::StructuredOutputWithToolsNotSupported,
        7 => ClientError::ImageNotSupported,
        8 => ClientError::ImageMediaTypeNotSupported {
            media_type: ImageMime::Gif,
        },
        _ => unreachable!("eight steps"),
    }
}

#[test]
fn given_a_call_that_breaks_no_rule_when_checked_then_it_passes() {
    assert_eq!(Scenario::passing().outcome(), Ok(()));
}

#[test]
fn given_a_call_that_breaks_one_rule_when_checked_then_that_step_refuses() {
    for step in STEPS {
        assert_eq!(
            Scenario::breaking(&[step]).outcome(),
            Err(refusal_of_step(step)),
            "step {step}"
        );
    }
}

#[test]
fn given_a_call_that_breaks_two_rules_when_checked_then_the_earlier_step_refuses() {
    for earlier in STEPS {
        for later in STEPS {
            if earlier >= later {
                continue;
            }
            assert_eq!(
                Scenario::breaking(&[earlier, later]).outcome(),
                Err(refusal_of_step(earlier)),
                "steps {earlier} and {later}"
            );
        }
    }
}

#[test]
fn given_a_call_that_breaks_the_rules_of_neighbouring_steps_when_checked_then_the_first_of_them_refuses()
 {
    for earlier in 1..8u8 {
        assert_eq!(
            Scenario::breaking(&[earlier, earlier + 1]).outcome(),
            Err(refusal_of_step(earlier)),
            "neighbours {earlier}"
        );
    }
}

#[test]
fn given_every_set_of_broken_rules_when_checked_then_the_lowest_step_of_the_set_refuses() {
    for mask in 1u16..256 {
        let steps: Vec<u8> = STEPS
            .iter()
            .copied()
            .filter(|step| mask & (1 << (step - 1)) != 0)
            .collect();
        let lowest = steps.iter().copied().min().unwrap();
        assert_eq!(
            Scenario::breaking(&steps).outcome(),
            Err(refusal_of_step(lowest)),
            "steps {steps:?}"
        );
    }
}

#[test]
fn given_a_call_with_an_image_when_images_are_off_and_the_media_type_is_also_bad_then_step_seven_refuses()
 {
    let capabilities = Capabilities::normalized(CapabilitiesSpec {
        images: false,
        image_media_types: Accepted::Only(vec![ImageMime::Png]),
        ..CapabilitiesSpec::everything()
    });
    assert_eq!(
        capabilities.check(
            &settings(ReasoningLevel::Off, CacheRequirement::NotRequired, 10),
            &image_request(&[ImageMime::Gif])
        ),
        Err(ClientError::ImageNotSupported)
    );
}

#[test]
fn given_every_step_broken_but_the_call_plain_when_checked_then_nothing_but_the_broken_rules_matter()
 {
    let capabilities = Capabilities::nothing();
    assert_eq!(
        capabilities.check(
            &settings(ReasoningLevel::Off, CacheRequirement::NotRequired, 1),
            &plain_request()
        ),
        Ok(())
    );
}
