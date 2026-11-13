//! The copied creature's printed text need not describe the optional copy ability.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn upkeep_choice_explains_the_copy() {
    let source = include_str!("../../../scenarios/vesuvan-doppelganger-copies-at-upkeep.cantrip")
        .split("Bot1.Accept\n[T3 Bot1.Main1]").next().unwrap().to_owned()
        + "[Handover Bot1.Accept]\n";
    let game = run(&GameScript::parse(&source, "vesuvan prompt").unwrap()).map_err(|f| f.report).unwrap().game;
    let RequestKind::May { purpose: Purpose::Ask(q), .. } = &game.pending().unwrap().kind else { panic!("expected described copy choice") };
    assert_eq!(q.verb, "Copy");
    assert_eq!(q.ask, "become a copy of the targeted creature?");
    assert_eq!(q.detail, "It keeps its color and this ability");
    assert_eq!(q.yes, "Become a copy");
    assert_eq!(q.no, "Keep current form");
}
