//! Flare selects the next turn, even if another upkeep occurs this turn or that turn's upkeep is skipped.
//! The engine's next_turn_delayed_tests exercise both timing cases for this timing kind.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn flare_schedules_the_next_turns_upkeep_and_describes_the_draw() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Black Lotus
Flare
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Flare.Cast(Bot2)
Bot1.Pass
Bot2.Pass
[Handover Bot1.Pass]
"#, "flare").unwrap();
    let result = run(&script).map_err(|f| f.report).unwrap();
    let game = result.game;
    assert_eq!(game.state.delayed.len(), 1);
    let delayed = &game.state.delayed[0];
    assert!(matches!(delayed.when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
    assert_eq!(delayed.controller, PlayerId(0));
    assert_eq!(game.cards.ability(delayed.ability).text(), "Draw a card at the beginning of the next turn's upkeep.");
}
