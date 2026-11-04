//! Gravebind draws on the next turn's upkeep, not merely the next upkeep (CR 603.7b).
use cantrip_engine::{DelayedWhen, Step};
use cantrip_script::{GameScript, run};

#[test]
fn gravebind_schedules_the_next_turns_upkeep() {
    let source = include_str!("../../../scenarios/gravebind-expires-before-next-turn.cantrip");
    let before_next_turn = source.split("[T2 Bot2.Main1]").next().unwrap();
    let script = GameScript::parse(before_next_turn, "gravebind-next-turn").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    assert_eq!(result.game.state.delayed.len(), 1);
    let delayed = &result.game.state.delayed[0];
    assert!(matches!(delayed.when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
    assert_eq!(result.game.cards.ability(delayed.ability).text(), "Draw a card at the beginning of the next turn's upkeep.");
}
