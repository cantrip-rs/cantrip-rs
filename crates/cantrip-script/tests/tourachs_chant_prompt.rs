use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn counter_choice_names_one_minus_one_counter() {
    let source = include_str!("../../../scenarios/tourachs-chant-upkeep-and-forests.cantrip")
        .split("Bot1.Choose(Ornithopter)").next().unwrap();
    let source = format!("{source}\n[Handover Bot1.Choose]\n");
    let script = GameScript::parse(&source, "tourachs-chant-prompt").unwrap();
    let result = run(&script).map_err(|f| f.report).unwrap();
    assert!(result.handed_over);
    let request = result.game.pending().expect("counter choice");
    assert_eq!(request.player, PlayerId(0));
    let RequestKind::ChooseCards { purpose, .. } = &request.kind else { panic!("expected creature choice") };
    assert!(matches!(purpose, Purpose::PutCounters(CounterKind::MinusOne, 1)), "wrong counter choice: {purpose:?}");
}
