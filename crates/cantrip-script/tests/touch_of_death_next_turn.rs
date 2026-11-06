//! Touch of Death schedules the next turn's upkeep, not the next upkeep (CR 603.7b).
use cantrip_engine::{DelayedWhen, Step};
use cantrip_script::{GameScript, run};

#[test]
fn touch_of_death_waits_for_the_next_turns_upkeep() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Touch of Death
Black Lotus
Swamp x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Hand.Touch of Death.Cast(Bot2)
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Life = 21
Bot2.Life = 19
"#, "touch of death next turn").unwrap();
    let result = run(&script).map_err(|f| f.report).unwrap();
    assert_eq!(result.game.state.delayed.len(), 1);
    let delayed = &result.game.state.delayed[0];
    assert_eq!(result.game.cards.ability(delayed.ability).text(),
        "Draw a card at the beginning of the next turn's upkeep.");
    assert!(matches!(delayed.when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
}
