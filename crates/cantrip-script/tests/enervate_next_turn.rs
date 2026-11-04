//! Enervate must select the next turn, not merely the next upkeep (CR 603.7b).
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn enervate_schedules_the_upkeep_of_the_next_turn() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Mox Sapphire
Mox Pearl
Mox Jet
Enervate
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Enervate.Cast(Bot1.Battlefield.Mox Jet)
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Battlefield.Mox Jet.Tapped = true
"#, "enervate-next-turn").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    let delayed = &result.game.state.delayed;
    assert_eq!(delayed.len(), 1);
    assert_eq!(result.game.cards.ability(delayed[0].ability).text(), "Draw a card at the beginning of the next turn's upkeep.");
    assert!(matches!(delayed[0].when, DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
}
