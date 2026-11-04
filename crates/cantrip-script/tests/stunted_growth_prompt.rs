use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn target_player_gets_an_ordered_put_on_top_choice() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Stunted Growth
Black Lotus
Mox Pearl
Mox Sapphire
Forest x20
[Bot2 Deck]
Island
Mountain
Swamp
Plains
Forest
Grizzly Bears
Lightning Bolt
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot1.Hand.Mox Pearl.Cast
Bot1.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Stunted Growth.Cast(Bot2)
[Handover Bot2.Choose]
"#, "stunted-growth-prompt").unwrap();
    let result = run(&script).map_err(|f| f.report).unwrap();
    assert!(result.handed_over);
    let request = result.game.pending().expect("choice");
    assert_eq!(request.player, PlayerId(1));
    assert!(matches!(&request.kind, RequestKind::ChooseCards { purpose: Purpose::PutOnTop, cards, .. } if cards.len() == 7));
}
