use br_llm_messages::ModelResponse;

use crate::build_error::MockBuildError;
use crate::sketch::Sketch;

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseInput {
    Response(Box<ModelResponse>),
    Sketch(Box<Sketch>),
}

impl ResponseInput {
    pub(crate) fn resolve(self) -> Result<ModelResponse, MockBuildError> {
        match self {
            ResponseInput::Response(response) => Ok(*response),
            ResponseInput::Sketch(sketch) => sketch.to_response(),
        }
    }
}

impl From<ModelResponse> for ResponseInput {
    fn from(response: ModelResponse) -> Self {
        ResponseInput::Response(Box::new(response))
    }
}

impl From<Sketch> for ResponseInput {
    fn from(sketch: Sketch) -> Self {
        ResponseInput::Sketch(Box::new(sketch))
    }
}

impl From<&ModelResponse> for ResponseInput {
    fn from(response: &ModelResponse) -> Self {
        ResponseInput::Response(Box::new(response.clone()))
    }
}

impl From<&Sketch> for ResponseInput {
    fn from(sketch: &Sketch) -> Self {
        ResponseInput::Sketch(Box::new(sketch.clone()))
    }
}
