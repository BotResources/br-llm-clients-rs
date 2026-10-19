use serde_json::{Value, json};

pub(crate) fn created(model: &str) -> Value {
    json!({
        "type": "response.created",
        "response": {
            "id": "resp_0b1c2d", "object": "response", "created_at": 1741290958,
            "status": "in_progress", "error": null, "incomplete_details": null,
            "model": model, "output": [], "store": false, "usage": null
        },
        "sequence_number": 0
    })
}

pub(crate) fn in_progress() -> Value {
    json!({ "type": "response.in_progress", "response": { "status": "in_progress" }, "sequence_number": 1 })
}

pub(crate) fn message_added(output_index: u64) -> Value {
    json!({
        "type": "response.output_item.added", "output_index": output_index,
        "item": { "id": "msg_0b1c", "type": "message", "status": "in_progress", "role": "assistant", "content": [] },
        "sequence_number": 2
    })
}

pub(crate) fn output_part() -> Value {
    json!({ "type": "output_text", "text": "", "annotations": [], "logprobs": [] })
}

pub(crate) fn refusal_part() -> Value {
    json!({ "type": "refusal", "refusal": "" })
}

pub(crate) fn part_added(output_index: u64, content_index: u64, part: Value) -> Value {
    json!({
        "type": "response.content_part.added", "item_id": "msg_0b1c",
        "output_index": output_index, "content_index": content_index,
        "part": part, "sequence_number": 3
    })
}

pub(crate) fn text_delta(output_index: u64, content_index: u64, delta: &str) -> Value {
    json!({
        "type": "response.output_text.delta", "item_id": "msg_0b1c",
        "output_index": output_index, "content_index": content_index,
        "delta": delta, "logprobs": [], "obfuscation": "xYz", "sequence_number": 4
    })
}

pub(crate) fn refusal_delta(output_index: u64, content_index: u64, delta: &str) -> Value {
    json!({
        "type": "response.refusal.delta", "item_id": "msg_0b1c",
        "output_index": output_index, "content_index": content_index,
        "delta": delta, "sequence_number": 4
    })
}

pub(crate) fn output_done_part(text: &str) -> Value {
    json!({ "type": "output_text", "text": text, "annotations": [], "logprobs": [] })
}

pub(crate) fn refusal_done_part(text: &str) -> Value {
    json!({ "type": "refusal", "refusal": text })
}

pub(crate) fn message_done(output_index: u64, status: Option<&str>, parts: Vec<Value>) -> Value {
    let mut item = json!({
        "id": "msg_0b1c", "type": "message", "role": "assistant", "content": parts
    });
    if let Some(status) = status {
        item["status"] = json!(status);
    }
    json!({ "type": "response.output_item.done", "output_index": output_index, "item": item, "sequence_number": 9 })
}

pub(crate) fn reasoning_added(output_index: u64) -> Value {
    json!({
        "type": "response.output_item.added", "output_index": output_index,
        "item": { "id": "rs_68a1", "type": "reasoning", "summary": [] },
        "sequence_number": 2
    })
}

pub(crate) fn summary_part_added(output_index: u64, summary_index: u64) -> Value {
    json!({
        "type": "response.reasoning_summary_part.added", "item_id": "rs_68a1",
        "output_index": output_index, "summary_index": summary_index,
        "part": { "type": "summary_text", "text": "" }, "sequence_number": 3
    })
}

pub(crate) fn summary_delta(output_index: u64, summary_index: u64, delta: &str) -> Value {
    json!({
        "type": "response.reasoning_summary_text.delta", "item_id": "rs_68a1",
        "output_index": output_index, "summary_index": summary_index,
        "delta": delta, "obfuscation": "q", "sequence_number": 4
    })
}

pub(crate) fn reasoning_text_delta(output_index: u64, content_index: u64, delta: &str) -> Value {
    json!({
        "type": "response.reasoning_text.delta", "item_id": "rs_68a1",
        "output_index": output_index, "content_index": content_index,
        "delta": delta, "sequence_number": 4
    })
}

pub(crate) fn reasoning_done(
    output_index: u64,
    id: &str,
    encrypted: Option<&str>,
    summaries: &[&str],
) -> Value {
    let summary: Vec<Value> = summaries
        .iter()
        .map(|text| json!({ "type": "summary_text", "text": text }))
        .collect();
    let mut item = json!({ "id": id, "type": "reasoning", "summary": summary });
    if let Some(encrypted) = encrypted {
        item["encrypted_content"] = json!(encrypted);
    }
    json!({ "type": "response.output_item.done", "output_index": output_index, "item": item, "sequence_number": 8 })
}

pub(crate) fn call_added(output_index: u64, call_id: &str, name: &str) -> Value {
    json!({
        "type": "response.output_item.added", "output_index": output_index,
        "item": {
            "id": "fc_77", "type": "function_call", "status": "in_progress",
            "arguments": "", "call_id": call_id, "name": name
        },
        "sequence_number": 2
    })
}

pub(crate) fn arguments_delta(output_index: u64, delta: &str) -> Value {
    json!({
        "type": "response.function_call_arguments.delta", "item_id": "fc_77",
        "output_index": output_index, "delta": delta, "obfuscation": "z", "sequence_number": 5
    })
}

pub(crate) fn call_done(
    output_index: u64,
    call_id: &str,
    name: &str,
    arguments: &str,
    status: Option<&str>,
) -> Value {
    let mut item = json!({
        "id": "fc_77", "type": "function_call",
        "arguments": arguments, "call_id": call_id, "name": name
    });
    if let Some(status) = status {
        item["status"] = json!(status);
    }
    json!({ "type": "response.output_item.done", "output_index": output_index, "item": item, "sequence_number": 7 })
}

pub(crate) fn usage_object(input: u64, output: u64) -> Value {
    json!({
        "input_tokens": input, "input_tokens_details": { "cached_tokens": 0 },
        "output_tokens": output, "output_tokens_details": { "reasoning_tokens": 0 },
        "total_tokens": input + output
    })
}

fn terminal(kind: &str, status: &str, model: &str, usage: Value, extra: Value) -> Value {
    let mut response = json!({
        "id": "resp_0b1c2d", "object": "response", "status": status, "model": model,
        "output": [], "usage": usage, "error": null, "incomplete_details": null
    });
    if let (Some(target), Some(source)) = (response.as_object_mut(), extra.as_object()) {
        target.extend(source.clone());
    }
    json!({ "type": kind, "response": response, "sequence_number": 13 })
}

pub(crate) fn completed(model: &str, usage: Value) -> Value {
    terminal("response.completed", "completed", model, usage, json!({}))
}

pub(crate) fn incomplete(reason: Option<&str>, usage: Value) -> Value {
    let details = match reason {
        Some(reason) => json!({ "reason": reason }),
        None => Value::Null,
    };
    terminal(
        "response.incomplete",
        "incomplete",
        "gpt-5.1-2026-01-01",
        usage,
        json!({ "incomplete_details": details }),
    )
}

pub(crate) fn failed(code: &str, message: &str) -> Value {
    terminal(
        "response.failed",
        "failed",
        "gpt-5.1-2026-01-01",
        Value::Null,
        json!({ "error": { "code": code, "message": message } }),
    )
}

pub(crate) fn error_event(code: Option<&str>, message: &str, param: Option<&str>) -> Value {
    json!({ "type": "error", "code": code, "message": message, "param": param, "sequence_number": 2 })
}
