use br_llm_messages::ImageMime;

use crate::capabilities_test_support::{
    CallShape, MODEL_NAME, STEPS, Step, image_request, settings,
};
use crate::{
    Accepted, CacheRequirement, Capabilities, CapabilitiesSpec, ClientError, ReasoningLevel,
};

const LIMIT: u32 = 100;

struct Scenario {
    spec: CapabilitiesSpec,
    call: CallShape,
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
            call: CallShape::plain(LIMIT),
        }
    }

    fn breaking(steps: &[Step]) -> Self {
        let mut scenario = Self::passing();
        for step in steps {
            scenario.break_step(*step);
        }
        scenario
    }

    fn break_step(&mut self, step: Step) {
        match step {
            Step::Reasoning => self.call.level = ReasoningLevel::High,
            Step::Cache => {
                self.spec.caching = false;
                self.call.cache = CacheRequirement::Required;
            }
            Step::Tokens => self.call.tokens = LIMIT + 1,
            Step::Tools => {
                self.spec.tools = false;
                self.call.tools = true;
            }
            Step::Structured => {
                self.spec.structured_output = false;
                self.call.structured = true;
            }
            Step::StructuredWithTools => {
                self.spec.structured_output_with_tools = false;
                self.call.structured = true;
                self.call.tools = true;
            }
            Step::Image => {
                self.spec.images = false;
                self.call.image = Some(ImageMime::Png);
            }
            Step::MediaType => {
                self.spec.image_media_types = Accepted::Only(vec![ImageMime::Png]);
                self.call.image = Some(ImageMime::Gif);
            }
        }
    }

    fn outcome(&self) -> Result<(), ClientError> {
        Capabilities::normalized(self.spec.clone())
            .check(&self.call.settings(), &self.call.request())
    }
}

fn refusal_of_step(step: Step) -> ClientError {
    match step {
        Step::Reasoning => ClientError::ReasoningNotSupported {
            level: ReasoningLevel::High,
            reason: format!(
                "the declared capabilities of model {MODEL_NAME} do not accept this level"
            ),
        },
        Step::Cache => ClientError::CacheNotSupported,
        Step::Tokens => ClientError::MaxOutputTokensTooLarge {
            limit: Some(u64::from(LIMIT)),
        },
        Step::Tools => ClientError::ToolsNotSupported,
        Step::Structured => ClientError::StructuredOutputNotSupported,
        Step::StructuredWithTools => ClientError::StructuredOutputWithToolsNotSupported,
        Step::Image => ClientError::ImageNotSupported,
        Step::MediaType => ClientError::ImageMediaTypeNotSupported {
            media_type: ImageMime::Gif,
        },
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
            "{step:?}"
        );
    }
}

#[test]
fn given_every_set_of_broken_rules_when_checked_then_the_lowest_step_of_the_set_refuses() {
    for mask in 1u16..256 {
        let steps: Vec<Step> = STEPS
            .iter()
            .enumerate()
            .filter(|(index, _)| mask & (1 << index) != 0)
            .map(|(_, step)| *step)
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
