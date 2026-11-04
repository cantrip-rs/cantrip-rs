//! Weaponry asks its controller whether to untap it.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn optional_untap_question_is_clear() {
    let script = GameScript::parse(r#"# Keeping Weaponry tapped preserves the bonus through an untap step; untapping it ends the bonus (CR 502.3, 611.2b).
# A noncreature is refused while a creature can be targeted with the same activation.
[Bot1 Deck]
Tawnos's Weaponry
Black Lotus
Mox Jet
Mox Pearl
Ornithopter
Island x20

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Ornithopter.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Hand.Tawnos's Weaponry.Cast
? Bot1.Mana = {U}
[T3 Bot1.Main1]
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Pearl.Activate
! Bot1.Battlefield.Tawnos's Weaponry.Activate(Bot1.Battlefield.Mox Jet)
Bot1.Battlefield.Tawnos's Weaponry.Activate(Bot1.Battlefield.Ornithopter)
Bot1.Choose(Pay mana)
Bot1.Choose(Pay mana)
? Bot1.Mana =
Bot1.Pass
Bot2.Pass
? Bot1.Battlefield.Ornithopter.Power = 1
? Bot1.Battlefield.Ornithopter.Toughness = 3
[Handover Bot1.Accept]
"#, "Weaponry untap").unwrap();
    let result = run(&script).unwrap_or_else(|f| panic!("{}", f.report));
    assert!(result.handed_over);
    let request = result.game.pending().unwrap();
    assert_eq!(request.player, PlayerId(0));
    let RequestKind::May { source, purpose: Purpose::Ask(question) } = &request.kind else { panic!("expected optional untap") };
    assert_eq!(result.game.cards.get(result.game.state.obj(*source).card).name, "Tawnos's Weaponry");
    assert_eq!(question.verb, "UntapInUntapStep");
    assert_eq!(question.ask, "untap it during this untap step?");
    assert_eq!(question.yes, "Untap it");
    assert_eq!(question.no, "Leave it tapped");
}
