//! Heal selects the next turn's upkeep, including when the current turn has an extra upkeep or the next turn skips it.
//! The engine's next_turn_delayed_tests exercise those timing cases for the selected kind.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn heal_schedules_the_next_turns_upkeep_and_describes_the_draw() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Mox Pearl
Heal
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Heal.Cast(Bot2)
Bot1.Pass
Bot2.Pass
[Handover Bot1.Pass]
"#, "heal").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    assert_eq!(game.state.delayed.len(), 1);
    let delayed = &game.state.delayed[0];
    assert!(matches!(delayed.when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
    assert_eq!(delayed.controller, PlayerId(0));
    assert_eq!(game.cards.ability(delayed.ability).text(), "Draw a card at the beginning of the next turn's upkeep.");
}
