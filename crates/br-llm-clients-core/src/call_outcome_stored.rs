use std::fmt;

use br_llm_messages::ModelResponse;
use serde::de::{self, Deserializer, IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Serialize, Serializer};

use crate::call_outcome::CallOutcome;
use crate::call_report::CallReport;
use crate::client_error::ClientError;
use crate::schema::{self, LoadError, SCHEMA_VERSION};

#[derive(Serialize)]
struct StoredCallOutcome<'a> {
    schema: &'static str,
    report: &'a CallReport,
    result: StoredResult<'a>,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum StoredResult<'a> {
    Success { response: &'a ModelResponse },
    Failure { error: &'a ClientError },
}

#[derive(Clone, Copy)]
enum Status {
    Success,
    Failure,
}

#[derive(Default)]
struct RawResult {
    status: Option<Status>,
    response: Option<ModelResponse>,
    error: Option<ClientError>,
    unknown: Option<String>,
}

struct ResultVisitor;

impl<'de> Visitor<'de> for ResultVisitor {
    type Value = RawResult;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a stored call result")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<RawResult, A::Error> {
        let mut raw = RawResult::default();
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "status" => {
                    if raw.status.is_some() {
                        return Err(de::Error::duplicate_field("status"));
                    }
                    let status = map.next_value::<String>()?;
                    raw.status = Some(match status.as_str() {
                        "success" => Status::Success,
                        "failure" => Status::Failure,
                        other => {
                            return Err(de::Error::unknown_variant(other, &["success", "failure"]));
                        }
                    });
                }
                "response" => set_once(&mut raw.response, "response", &mut map)?,
                "error" => set_once(&mut raw.error, "error", &mut map)?,
                _ => {
                    map.next_value::<IgnoredAny>()?;
                    raw.unknown.get_or_insert(key);
                }
            }
        }
        Ok(raw)
    }
}

impl<'de> Deserialize<'de> for RawResult {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ResultVisitor)
    }
}

impl Serialize for CallOutcome {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let result = match &self.result {
            Ok(response) => StoredResult::Success { response },
            Err(error) => StoredResult::Failure { error },
        };
        StoredCallOutcome {
            schema: SCHEMA_VERSION,
            report: &self.report,
            result,
        }
        .serialize(serializer)
    }
}

fn assemble(report: CallReport, raw: RawResult) -> Result<CallOutcome, LoadError> {
    let wrong = |status, problem| LoadError::Result { status, problem };
    match (raw.status, raw.response, raw.error) {
        (None, _, _) => Err(wrong("missing", "has no status")),
        (Some(Status::Success), Some(response), None) => {
            Ok(CallOutcome::success(report, response)?)
        }
        (Some(Status::Failure), None, Some(error)) => Ok(CallOutcome::failure(report, error)),
        (Some(Status::Success), None, _) => Err(wrong("success", "holds no response")),
        (Some(Status::Success), Some(_), Some(_)) => Err(wrong("success", "holds an error")),
        (Some(Status::Failure), _, None) => Err(wrong("failure", "holds no error")),
        (Some(Status::Failure), Some(_), Some(_)) => Err(wrong("failure", "holds a response")),
    }
}

struct OutcomeVisitor;

impl<'de> Visitor<'de> for OutcomeVisitor {
    type Value = CallOutcome;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a stored call outcome")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<CallOutcome, A::Error> {
        let mut schema: Option<String> = None;
        let mut report: Option<CallReport> = None;
        let mut result: Option<RawResult> = None;
        let mut unknown: Option<String> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "schema" => set_once(&mut schema, "schema", &mut map)?,
                "report" => set_once(&mut report, "report", &mut map)?,
                "result" => set_once(&mut result, "result", &mut map)?,
                _ => {
                    map.next_value::<IgnoredAny>()?;
                    unknown.get_or_insert(key);
                }
            }
        }
        let schema = schema.ok_or_else(|| de::Error::missing_field("schema"))?;
        schema::check(&schema).map_err(de::Error::custom)?;
        let report = report.ok_or_else(|| de::Error::missing_field("report"))?;
        let result = result.ok_or_else(|| de::Error::missing_field("result"))?;
        if let Some(key) = unknown {
            return Err(de::Error::unknown_field(
                &key,
                &["schema", "report", "result"],
            ));
        }
        if let Some(key) = &result.unknown {
            return Err(de::Error::unknown_field(
                key,
                &["status", "response", "error"],
            ));
        }
        assemble(report, result).map_err(de::Error::custom)
    }
}

fn set_once<'de, T: Deserialize<'de>, A: MapAccess<'de>>(
    slot: &mut Option<T>,
    name: &'static str,
    map: &mut A,
) -> Result<(), A::Error> {
    if slot.is_some() {
        return Err(de::Error::duplicate_field(name));
    }
    *slot = Some(map.next_value()?);
    Ok(())
}

impl<'de> Deserialize<'de> for CallOutcome {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_struct(
            "CallOutcome",
            &["schema", "report", "result"],
            OutcomeVisitor,
        )
    }
}
