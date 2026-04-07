use crate::scorer::assertion_record::AssertionRecord;

pub fn print_cli_report(record: &AssertionRecord) {
    println!("VibeDiff Audit Result");
    println!("score: {:.2}", record.composite_score);
    println!("label: {:?}", record.label);
    println!(
        "dimensions: logic={:.2} scope={:.2} side_effect={:.2} structural={:.2}",
        record.scores.logic_match,
        record.scores.scope_adherence,
        record.scores.side_effect_detection,
        record.scores.structural_proportionality
    );
}
