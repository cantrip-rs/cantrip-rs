//! Formation must select the next turn, not merely the next upkeep (CR 603.7b).
use cantrip_engine::{DelayedWhen, Step};
use cantrip_script::{GameScript, run};

#[test]
fn formation_waits_for_the_next_turns_upkeep() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Black Lotus
Formation
Ornithopter
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Ornithopter.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Hand.Formation.Cast(Bot1.Battlefield.Ornithopter)
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Battlefield.Ornithopter.Keywords = Banding; Flying
"#, "formation next turn").unwrap();
    let result = run(&script).map_err(|f| f.report).unwrap();
    assert_eq!(result.game.state.delayed.len(), 1);
    assert_eq!(result.game.cards.ability(result.game.state.delayed[0].ability).text(),
        "Draw a card at the beginning of the next turn’s upkeep.");
    assert!(matches!(result.game.state.delayed[0].when,
        DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None }));
}
