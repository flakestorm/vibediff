use vibediff_core::scorer::assertion_record::{DimensionScores, VibeLabel};

#[test]
fn composite_score_uses_spec_weights() {
    let s = DimensionScores {
        logic_match: 1.0,
        scope_adherence: 0.0,
        side_effect_detection: 0.0,
        structural_proportionality: 0.0,
    };
    let c = s.composite();
    assert!((c - 0.35).abs() < 1e-9);
}

#[test]
fn score_to_label_matches_ranges() {
    assert!(matches!(VibeLabel::from_score(0.9), VibeLabel::Aligned));
    assert!(matches!(VibeLabel::from_score(0.75), VibeLabel::Drifting));
    assert!(matches!(VibeLabel::from_score(0.55), VibeLabel::Suspect));
    assert!(matches!(VibeLabel::from_score(0.25), VibeLabel::Misaligned));
}
