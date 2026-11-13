//! Cursed Rack's opponent choice carries a descriptive hint, not a numeric amount.
use cantrip_engine::*;
use cantrip_script::{Advance, GameScript, Scripted};

#[test]
fn cursed_rack_opponent_choice_metadata() {
    let text = include_str!("../../../scenarios/cursed-rack-chosen-opponent-discards-to-four.cantrip");
    let script = GameScript::parse(text, "cursed-rack-prompt-metadata").unwrap();
    let mut table = Scripted::deal(script).unwrap();
    let players = [PlayerId(0), PlayerId(1), PlayerId(2)];
    let mut choices = 0;
    for _ in 0..1000 {
        match table.advance(&players).unwrap() {
            Advance::Ask(expected) => {
                if let RequestKind::ChoosePlayer { source, candidates, verb, amount, subject } = &expected.request.kind {
                    assert_eq!(table.game().name(*source), "Cursed Rack");
                    assert_eq!(expected.request.player, PlayerId(0));
                    assert_eq!(candidates, &[PlayerId(1), PlayerId(2)]);
                    assert_eq!(*amount, 0, "the hint has no separate numeric amount");
                    assert_eq!(*subject, None);
                    assert_eq!(*verb, "has a maximum hand size of four");
                    choices += 1;
                }
                table.answered(expected.intent).unwrap();
            }
            Advance::Check(_) => {}
            Advance::Done => {
                table.verdict().unwrap();
                assert_eq!(choices, 1);
                return;
            }
        }
    }
    panic!("the scenario did not finish");
}
