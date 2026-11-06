use cantrip_engine::{PlayerId, Purpose, RequestKind};
use cantrip_script::{GameScript, run};

#[test]
fn paid_trigger_asks_for_a_card_from_the_casters_hand() {
    let text = include_str!("../../../scenarios/leshracs-sigil-pays-to-discard-from-green-spell-caster.cantrip");
    let prefix = text.split("! Bot1.Choose()").next().unwrap();
    let script = GameScript::parse(&format!("{prefix}[Handover Bot1.Choose]\n"), "sigil-choice").unwrap();
    let result = run(&script).unwrap();
    let request = result.game.pending().expect("the hand choice");
    assert_eq!(request.player, PlayerId(0));
    let RequestKind::ChooseCards { purpose, cards, .. } = &request.kind else { panic!("expected card choice") };
    assert_eq!(*purpose, Purpose::TakeFromHand);
    assert!(!cards.is_empty());
    assert!(cards.iter().all(|&card| result.game.state.obj(card).owner == PlayerId(1)));
    assert!(cards.iter().any(|&card| result.game.name(card) == "Lightning Bolt"));
}
