use cantrip_script::{GameScript, run};

#[test]
fn no_extra_discard_is_offered_after_paying_for_all_life_lost() {
    let source = include_str!("../../../scenarios/oath-of-lim-dul-life-loss-and-opponent.cantrip");
    let script = GameScript::parse(source, "oath-payment-boundary").unwrap();
    let result = run(&script).map_err(|failure| failure.report).unwrap();
    assert!(result.trace.iter().all(|line| !(line.contains("(default)") && line.contains("UnlessPays"))),
        "no third discard payment should be offered: {:?}", result.trace);
}
