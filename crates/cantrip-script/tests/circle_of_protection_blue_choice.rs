//! The source choice retains its color, purpose and required count for the client.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn blue_source_choice_metadata() {
    let script = GameScript::parse(r#"# A blue spell on the stack is an eligible source; its damage is prevented (CR 609.7a).
[Bot1 Deck]
Mox Pearl
Mox Ruby
Mox Jet
Circle of Protection: Blue
Island x20
[Bot2 Deck]
Black Lotus
Psionic Blast
Island x20
[Actions]
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Circle of Protection: Blue.Cast
[T2 Bot2.Main1]
Bot2.Hand.Black Lotus.Cast
Bot2.Battlefield.Black Lotus.Activate(Color: Blue)
Bot2.Hand.Psionic Blast.Cast(Bot1)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Circle of Protection: Blue.Activate
Bot1.Pass
Bot2.Pass
[Handover Bot1.Choose]
"#, "blue source choice").unwrap();
    let result = run(&script).unwrap_or_else(|f| panic!("{}", f.report));
    assert!(result.handed_over);
    let request = result.game.pending().unwrap();
    let RequestKind::ChooseCards { purpose, rule, cards, .. } = &request.kind else { panic!("expected a card choice") };
    assert_eq!(*purpose, Purpose::Choose);
    assert_eq!(rule.count, Count::AsManyAsCan(1));
    assert_eq!(rule.kind, "source of damage");
    assert_eq!(rule.words(&|_| String::new()), ["blue"]);
    assert!(cards.iter().any(|id| result.game.cards.get(result.game.state.obj(*id).card).name == "Psionic Blast"));
}
