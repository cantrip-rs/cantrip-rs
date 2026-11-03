use super::damage_for_toughness;

#[test]
fn missing_toughness_is_zero() {
    // A creature with no toughness value has toughness 0 (CR 208.5).
    assert_eq!(damage_for_toughness(None), 0);
    assert_eq!(damage_for_toughness(Some(-1)), 0);
    assert_eq!(damage_for_toughness(Some(5)), 5);
}
