use serde_json::json;

use crate::scorer::assertion_record::AssertionRecord;

pub fn to_sarif(record: &AssertionRecord) -> serde_json::Value {
    let level = match format!("{:?}", record.label).as_str() {
        "Misaligned" => "error",
        "Suspect" => "warning",
        _ => "note",
    };
    json!({
      "version": "2.1.0",
      "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
      "runs": [{
        "tool": {"driver": {"name": "vibediff", "version": env!("CARGO_PKG_VERSION")}},
        "results": [{
          "ruleId": "vibediff.semantic-alignment",
          "level": level,
          "message": {"text": format!("VibeDiff score={:.2} label={:?}", record.composite_score, record.label)}
        }]
      }]
    })
}
