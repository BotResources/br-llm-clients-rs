mod depth;
mod parameter;
mod parameters;
mod spec;

#[cfg(test)]
mod depth_tests;
#[cfg(test)]
mod parameter_tests;
#[cfg(test)]
mod spec_tests;

pub use depth::MAX_NESTING_DEPTH;
pub(crate) use depth::{check_depth, check_kind_depth};
pub use parameter::{Parameter, ParameterKind};
pub use parameters::Parameters;
pub use spec::ToolSpec;

pub(crate) fn is_blank(value: &str) -> bool {
    value.trim().is_empty()
}
