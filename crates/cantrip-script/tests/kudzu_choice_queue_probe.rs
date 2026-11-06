use cantrip_engine::*;
use cantrip_script::{Advance, GameScript, Scripted};
#[test]
fn inspect_queue_when_kudzu_offers_a_land() {
    let script = GameScript::parse(include_str!("../../../scenarios/kudzu-phase-return-jihad-boundary.cantrip"), "kudzu-choice").unwrap();
    let mut driven = Scripted::deal(script).unwrap();
    loop {
        match driven.advance(&[PlayerId(0)]).unwrap() {
            Advance::Ask(expected) => {
                if expected.line.as_deref().is_some_and(|s| s.contains("Bot1.Choose(Bot2.Battlefield.Swamp)")) {
                    let game = driven.game();
                    assert!(game.state.triggers.iter().any(|t| game.cards.get(game.state.obj(t.source).card).name == "Jihad"));
                    break;
                }
                driven.answered(expected.intent).unwrap();
            }
            Advance::Check(_) => {},
            Advance::Done => panic!("choice not reached"),
        }
    }
}
