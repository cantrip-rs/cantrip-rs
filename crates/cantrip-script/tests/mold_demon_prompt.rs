use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn mold_demon_names_its_sacrifice_choice() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Swamp x2
Black Lotus x2
Mox Jet
Mold Demon
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Swamp.Play
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Jet.Cast
[T3 Bot1.Main1]
Bot1.Hand.Swamp.Play
Bot1.Battlefield.Black Lotus#1.Activate(Color: Black)
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Mold Demon.Cast
Bot1.Pass
Bot2.Pass
Bot1.Pass
Bot2.Pass
[Handover Bot1.Decline]
"#, "mold-demon-prompt").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let request = game.pending().expect("payment question");
    let RequestKind::UnlessPays { question, options, .. } = &request.kind else { panic!("expected payment question") };
    assert_eq!(question.ask, "sacrifice two lands matching this ability?");
    assert_eq!(question.yes, "Sacrifice two lands");
    assert!(options.iter().any(|o| o.kind == ActionKind::PayCost));
    assert!(options.iter().any(|o| o.kind == ActionKind::DeclineCost));
}
