use std::fmt;

use br_llm_messages::ModelResponse;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize, Serializer};

use crate::call_outcome::CallOutcome;
use crate::call_report::CallReport;
use crate::client_error::ClientError;
use crate::schema::{self, LoadError, SCHEMA_VERSION, present};

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

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Success,
    Failure,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResult {
    status: Status,
    #[serde(default, deserialize_with = "present")]
    response: Option<ModelResponse>,
    #[serde(default, deserialize_with = "present")]
    error: Option<ClientError>,
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
        (Status::Success, Some(response), None) => Ok(CallOutcome::success(report, response)?),
        (Status::Failure, None, Some(error)) => Ok(CallOutcome::failure(report, error)),
        (Status::Success, None, _) => Err(wrong("success", "holds no response")),
        (Status::Success, Some(_), Some(_)) => Err(wrong("success", "holds an error")),
        (Status::Failure, _, None) => Err(wrong("failure", "holds no error")),
        (Status::Failure, Some(_), Some(_)) => Err(wrong("failure", "holds a response")),
    }
}

struct OutcomeVisitor;

impl<'de> Visitor<'de> for OutcomeVisitor {
    type Value = CallOutcome;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a stored call outcome")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<CallOutcome, A::Error> {
        let mut schema_seen = false;
        let mut report: Option<CallReport> = None;
        let mut result: Option<RawResult> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "schema" => {
                    if schema_seen {
                        return Err(de::Error::duplicate_field("schema"));
                    }
                    let found: String = map.next_value()?;
                    schema::check(&found).map_err(de::Error::custom)?;
                    schema_seen = true;
                }
                "report" => {
                    if report.is_some() {
                        return Err(de::Error::duplicate_field("report"));
                    }
                    report = Some(map.next_value()?);
                }
                "result" => {
                    if result.is_some() {
                        return Err(de::Error::duplicate_field("result"));
                    }
                    result = Some(map.next_value()?);
                }
                _ => {
                    return Err(de::Error::unknown_field(
                        &key,
                        &["schema", "report", "result"],
                    ));
                }
            }
        }
        if !schema_seen {
            return Err(de::Error::missing_field("schema"));
        }
        let report = report.ok_or_else(|| de::Error::missing_field("report"))?;
        let result = result.ok_or_else(|| de::Error::missing_field("result"))?;
        assemble(report, result).map_err(de::Error::custom)
    }
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
