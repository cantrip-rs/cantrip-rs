//! The targeted player names any card without seeing the library card first.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn targeted_player_names_without_revealing_first() {
    let script = GameScript::parse(r#"# The targeted player names before the reveal: Bot2 names its Forest and gets it, while
# Bot1 names Island and puts the different top card into its graveyard (CR 701.20a).
[Bot1 Deck]
Black Lotus
Mox Pearl
Mox Sapphire
Petra Sphinx
Plains
Plains
Plains
Mountain
Swamp
Forest
Island x20

[Bot2 Deck]
Island x7
Swamp
Forest
Island x20

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Petra Sphinx.Cast
? Bot1.Mana =
[T3 Bot1.Main1]
! Bot1.Battlefield.Petra Sphinx.Activate(Bot1.Battlefield.Mox Pearl)
Bot1.Battlefield.Petra Sphinx.Activate(Bot2)
? Bot1.Battlefield.Petra Sphinx.Tapped = true
Bot1.Pass
Bot2.Pass
? Bot2.Revealed = (absent)
[Handover Bot2.Name]
"#, "Petra naming").unwrap();
    let result = run(&script).unwrap_or_else(|f| panic!("{}", f.report));
    assert!(result.handed_over);
    let request = result.game.pending().unwrap();
    assert_eq!(request.player, PlayerId(1));
    let RequestKind::ChooseName { looked_at, suggestion, kind, .. } = &request.kind else { panic!("expected naming prompt") };
    assert!(looked_at.is_empty());
    assert_eq!(*suggestion, None);
    assert_eq!(*kind, NameKind::ANY);
}
