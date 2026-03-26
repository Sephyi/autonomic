//! Stream-JSON output parser for Claude Code's `--output-format stream-json`.
//!
//! Claude Code emits one JSON object per line on stdout when invoked with
//! `--output-format stream-json`. This module parses those lines and
//! classifies the final result message into a `SessionOutcome`.

use serde::Deserialize;

/// A single message from Claude Code's stream-json output.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamMessage {
    /// The message type (e.g., "assistant", "result", "system").
    #[serde(rename = "type")]
    pub message_type: String,

    /// The message subtype (e.g., "success", "error", "max_turns").
    pub subtype: Option<String>,

    /// Cumulative cost in USD at this point in the session.
    pub total_cost_usd: Option<f64>,

    /// Elapsed time in milliseconds.
    pub duration_ms: Option<u64>,

    /// The Claude session ID assigned by the API.
    pub session_id: Option<String>,

    /// The result text, if this is a result message.
    pub result: Option<String>,

    /// The error description, if this is an error message.
    pub error: Option<String>,

    /// Any additional fields not explicitly modeled.
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// The classified outcome of a completed Claude Code session.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionOutcome {
    /// The session completed successfully.
    Success {
        /// The final result text from Claude.
        result_text: String,
        /// Total cost in USD.
        cost_usd: f64,
        /// Total duration in milliseconds.
        duration_ms: u64,
        /// The Claude session ID.
        claude_session_id: String,
    },
    /// The session ended with an error.
    Error {
        /// The error description.
        error: String,
        /// Total cost incurred before the error.
        cost_usd: f64,
    },
    /// The session hit the maximum number of turns.
    MaxTurns {
        /// The partial result text.
        result_text: String,
        /// Total cost in USD.
        cost_usd: f64,
        /// The Claude session ID.
        claude_session_id: String,
    },
}

/// Parse a single line of stream-json output.
///
/// Returns `Ok(None)` for empty or whitespace-only lines.
/// Returns `Ok(Some(msg))` for valid JSON lines.
/// Returns `Err(description)` for lines that fail to parse.
pub fn parse_line(line: &str) -> Result<Option<StreamMessage>, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    serde_json::from_str(trimmed)
        .map(Some)
        .map_err(|e| e.to_string())
}

/// Classify a result-type `StreamMessage` into a `SessionOutcome`.
///
/// This should be called on messages where `message_type == "result"`.
pub fn classify_result(msg: &StreamMessage) -> SessionOutcome {
    let cost_usd = msg.total_cost_usd.unwrap_or(0.0);
    let duration_ms = msg.duration_ms.unwrap_or(0);
    let session_id = msg.session_id.clone().unwrap_or_default();

    match msg.subtype.as_deref() {
        Some("error") => SessionOutcome::Error {
            error: msg
                .error
                .clone()
                .unwrap_or_else(|| "unknown error".to_string()),
            cost_usd,
        },
        Some("max_turns") => SessionOutcome::MaxTurns {
            result_text: msg.result.clone().unwrap_or_default(),
            cost_usd,
            claude_session_id: session_id,
        },
        // "success" or any other/unknown subtype treated as success
        _ => SessionOutcome::Success {
            result_text: msg.result.clone().unwrap_or_default(),
            cost_usd,
            duration_ms,
            claude_session_id: session_id,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_line_returns_none() {
        assert!(parse_line("").unwrap().is_none());
        assert!(parse_line("   ").unwrap().is_none());
        assert!(parse_line("\n").unwrap().is_none());
    }

    #[test]
    fn parse_assistant_text_message() {
        let line = r#"{"type":"assistant","message":"Hello"}"#;
        let msg = parse_line(line).unwrap().unwrap();
        assert_eq!(msg.message_type, "assistant");
        assert!(msg.subtype.is_none());
    }

    #[test]
    fn parse_result_success_message() {
        let line = r#"{"type":"result","subtype":"success","result":"Done","total_cost_usd":0.05,"duration_ms":1200,"session_id":"sess-abc"}"#;
        let msg = parse_line(line).unwrap().unwrap();
        assert_eq!(msg.message_type, "result");
        assert_eq!(msg.subtype.as_deref(), Some("success"));
        assert_eq!(msg.result.as_deref(), Some("Done"));
        assert!((msg.total_cost_usd.unwrap() - 0.05).abs() < f64::EPSILON);
        assert_eq!(msg.duration_ms, Some(1200));
        assert_eq!(msg.session_id.as_deref(), Some("sess-abc"));
    }

    #[test]
    fn parse_result_error_message() {
        let line =
            r#"{"type":"result","subtype":"error","error":"Rate limited","total_cost_usd":0.01}"#;
        let msg = parse_line(line).unwrap().unwrap();
        assert_eq!(msg.message_type, "result");
        assert_eq!(msg.subtype.as_deref(), Some("error"));
        assert_eq!(msg.error.as_deref(), Some("Rate limited"));
    }

    #[test]
    fn parse_tool_use_message() {
        let line = r#"{"type":"tool_use","tool":"bash","input":"ls -la"}"#;
        let msg = parse_line(line).unwrap().unwrap();
        assert_eq!(msg.message_type, "tool_use");
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let result = parse_line("{not valid json}");
        assert!(result.is_err());
    }

    #[test]
    fn classify_success_result() {
        let msg = StreamMessage {
            message_type: "result".to_string(),
            subtype: Some("success".to_string()),
            total_cost_usd: Some(0.10),
            duration_ms: Some(5000),
            session_id: Some("sess-123".to_string()),
            result: Some("All done".to_string()),
            error: None,
            extra: serde_json::Value::Object(serde_json::Map::new()),
        };
        let outcome = classify_result(&msg);
        assert_eq!(
            outcome,
            SessionOutcome::Success {
                result_text: "All done".to_string(),
                cost_usd: 0.10,
                duration_ms: 5000,
                claude_session_id: "sess-123".to_string(),
            }
        );
    }

    #[test]
    fn classify_error_result() {
        let msg = StreamMessage {
            message_type: "result".to_string(),
            subtype: Some("error".to_string()),
            total_cost_usd: Some(0.02),
            duration_ms: None,
            session_id: None,
            result: None,
            error: Some("API error".to_string()),
            extra: serde_json::Value::Object(serde_json::Map::new()),
        };
        let outcome = classify_result(&msg);
        assert_eq!(
            outcome,
            SessionOutcome::Error {
                error: "API error".to_string(),
                cost_usd: 0.02,
            }
        );
    }

    #[test]
    fn classify_max_turns_result() {
        let msg = StreamMessage {
            message_type: "result".to_string(),
            subtype: Some("max_turns".to_string()),
            total_cost_usd: Some(1.50),
            duration_ms: Some(60000),
            session_id: Some("sess-456".to_string()),
            result: Some("Partial work".to_string()),
            error: None,
            extra: serde_json::Value::Object(serde_json::Map::new()),
        };
        let outcome = classify_result(&msg);
        assert_eq!(
            outcome,
            SessionOutcome::MaxTurns {
                result_text: "Partial work".to_string(),
                cost_usd: 1.50,
                claude_session_id: "sess-456".to_string(),
            }
        );
    }

    #[test]
    fn classify_unknown_subtype() {
        let msg = StreamMessage {
            message_type: "result".to_string(),
            subtype: Some("unknown_variant".to_string()),
            total_cost_usd: Some(0.0),
            duration_ms: Some(0),
            session_id: Some(String::new()),
            result: Some("result".to_string()),
            error: None,
            extra: serde_json::Value::Object(serde_json::Map::new()),
        };
        // Unknown subtypes are treated as success
        let outcome = classify_result(&msg);
        assert!(matches!(outcome, SessionOutcome::Success { .. }));
    }

    #[test]
    fn extra_fields_captured() {
        let line = r#"{"type":"assistant","custom_field":"custom_value","nested":{"a":1}}"#;
        let msg = parse_line(line).unwrap().unwrap();
        assert_eq!(msg.extra["custom_field"], "custom_value");
        assert_eq!(msg.extra["nested"]["a"], 1);
    }

    #[test]
    fn serde_roundtrip_stream_message_fields() {
        let line = r#"{"type":"result","subtype":"success","total_cost_usd":0.123,"duration_ms":999,"session_id":"s1","result":"ok","error":null}"#;
        let msg = parse_line(line).unwrap().unwrap();
        assert_eq!(msg.message_type, "result");
        assert_eq!(msg.subtype.as_deref(), Some("success"));
        assert!((msg.total_cost_usd.unwrap() - 0.123).abs() < f64::EPSILON);
        assert_eq!(msg.duration_ms, Some(999));
        assert_eq!(msg.session_id.as_deref(), Some("s1"));
        assert_eq!(msg.result.as_deref(), Some("ok"));
        assert!(msg.error.is_none());
    }
}
