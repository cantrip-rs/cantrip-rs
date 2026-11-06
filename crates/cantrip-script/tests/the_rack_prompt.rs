//! The Rack's actual entering choice supplies descriptive text, without a numeric
//! amount: nonzero amounts are observable to clients and would misstate this choice.
use cantrip_engine::*;
use cantrip_script::{Advance, GameScript, Scripted};

#[test]
fn the_racks_entering_opponent_choice_has_no_numeric_amount_or_target_subject() {
    let text = include_str!("../../../scenarios/the-rack-chosen-opponent-leaves-the-game.cantrip");
    let script = GameScript::parse(text, "the-rack-prompt-metadata").unwrap();
    let mut table = Scripted::deal(script).unwrap();
    let players = [PlayerId(0), PlayerId(1), PlayerId(2)];
    let mut choices = 0;
    for _ in 0..1000 {
        match table.advance(&players).unwrap() {
            Advance::Ask(expected) => {
                if let RequestKind::ChoosePlayer { source, candidates, verb, amount, subject } = &expected.request.kind {
                    assert_eq!(table.game().name(*source), "The Rack");
                    assert_eq!(expected.request.player, PlayerId(0));
                    assert_eq!(candidates, &[PlayerId(1), PlayerId(2)]);
                    assert_eq!(*amount, 0, "this descriptive opponent choice has no numeric amount");
                    assert_eq!(*subject, None, "the entering choice does not target a subject");
                    assert_eq!(*verb, "takes damage at the beginning of their upkeep based on the number of cards in their hand");
                    choices += 1;
                }
                table.answered(expected.intent).unwrap();
            }
            Advance::Done => {
                table.verdict().unwrap();
                assert_eq!(choices, 1, "the actual entering handler must ask its opponent choice");
                return;
            }
            Advance::Check(_) => panic!("this forced scenario contains no client checks"),
        }
    }
    panic!("the forced scenario did not finish");
}
