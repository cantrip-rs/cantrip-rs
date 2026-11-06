//! The next-turn timing kind handles extra current-turn and skipped next-turn upkeeps.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn clairvoyance_schedules_the_next_turns_upkeep() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Mox Sapphire
Clairvoyance
Island x20
[Bot2 Deck]
Forest x20
[Actions]
Bot1.Hand.Mox Sapphire.Cast
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Clairvoyance.Cast(Bot2)
Bot1.Pass
Bot2.Pass
[Handover Bot1.Pass]
"#, "clairvoyance").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    assert_eq!(game.state.delayed.len(), 1);
    let delayed = &game.state.delayed[0];
    assert!(matches!(delayed.when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
    assert_eq!(delayed.controller, PlayerId(0));
    assert_eq!(game.cards.ability(delayed.ability).text(), "Draw a card at the beginning of the next turn's upkeep.");
}
