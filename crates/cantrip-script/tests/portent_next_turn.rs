//! Portent schedules the next turn's upkeep, not the next upkeep (CR 603.7b).
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn portent_schedules_the_upkeep_of_the_next_turn() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Portent
Mox Sapphire
Island x5
Plains
Swamp
Mountain
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Portent.Cast(Bot1)
Bot1.Pass
Bot2.Pass
Bot1.Arrange(Mountain; Plains; Swamp)
Bot1.Decline
[Expect]
Bot1.Library.Count = 23
"#, "portent-next-turn").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    let delayed = &result.game.state.delayed;
    assert_eq!(delayed.len(), 1);
    assert!(matches!(delayed[0].when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
    assert_eq!(result.game.cards.ability(delayed[0].ability).text(), "Draw a card at the beginning of the next turn's upkeep.");
}
