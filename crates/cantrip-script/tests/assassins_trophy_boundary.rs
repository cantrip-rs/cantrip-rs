//! The destroyed permanent's return effects are judged before its controller chooses whether to search (CR 603.8, 608.2c).
use cantrip_engine::*;
use cantrip_script::{Advance, GameScript, Scripted};

#[test]
fn phased_in_jihad_is_queued_before_the_controller_is_asked_to_search() {
    let text = include_str!("../../../scenarios/phased-jihad-triggers-before-a-forest-enters.cantrip");
    let script = GameScript::parse(text, "trophy-search-boundary").unwrap();
    let mut driven = Scripted::deal(script).unwrap();
    loop {
        match driven.advance(&[PlayerId(1)]).unwrap() {
            Advance::Ask(expected) => {
                if expected.line.as_deref().is_some_and(|s| s.contains("Bot2.Accept")) {
                    let g = driven.game();
                    assert!(g.state.triggers.iter().any(|t| g.cards.get(g.state.obj(t.source).card).name == "Jihad"),
                        "Jihad must trigger before the search choice");
                    break;
                }
                driven.answered(expected.intent).unwrap();
            }
            Advance::Check(_) => {},
            Advance::Done => panic!("search choice was not reached"),
        }
    }
}
