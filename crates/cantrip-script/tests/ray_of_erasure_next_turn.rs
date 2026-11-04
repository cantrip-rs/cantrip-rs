//! Ray of Erasure selects the next turn's upkeep, not the next upkeep (CR 603.7b).
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn ray_of_erasure_schedules_the_next_turns_upkeep() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Mox Sapphire
Ray of Erasure
Island x20
[Bot2 Deck]
Island x7
Forest
Island x20
[Actions]
Bot1.Hand.Mox Sapphire.Cast
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Ray of Erasure.Cast(Bot2)
Bot1.Pass
Bot2.Pass
[Expect]
Bot2.Graveyard = Forest
"#, "ray-of-erasure-next-turn").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    let delayed = &result.game.state.delayed;
    assert_eq!(delayed.len(), 1);
    assert!(matches!(delayed[0].when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
    assert_eq!(result.game.cards.ability(delayed[0].ability).text(), "Draw a card at the beginning of the next turn's upkeep.");
}
