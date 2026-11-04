//! Pyknite selects the next turn's upkeep, not the next upkeep (CR 603.7).
use cantrip_engine::*;
use cantrip_script::{run, GameScript};

#[test]
fn pyknite_schedules_the_next_turns_upkeep() {
    let source = include_str!("../../../scenarios/pyknite-draws-once-in-the-next-turns-upkeep.cantrip");
    let before_next_turn = source.split("[T2 Bot2.Upkeep]").next().unwrap();
    let script = GameScript::parse(before_next_turn, "pyknite-timing").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    assert_eq!(result.game.state.delayed.len(), 1);
    let delayed = &result.game.state.delayed[0];
    assert!(matches!(delayed.when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
    assert_eq!(result.game.cards.ability(delayed.ability).text(), "Draw a card at the beginning of the next turn's upkeep.");
}
