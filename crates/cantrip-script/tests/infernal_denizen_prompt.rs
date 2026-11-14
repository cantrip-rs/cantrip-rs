//! The multiplayer opponent choice describes the actual optional control effect without a spurious numeric amount.
use cantrip_engine::{PlayerId, RequestKind};
use cantrip_script::{Advance, GameScript, Scripted};

#[test]
fn opponent_choice_describes_optional_control() {
    let text = include_str!("../../../scenarios/infernal-denizen-controller-chooses-opponent.cantrip");
    let mut table = Scripted::deal(GameScript::parse(text, "infernal-denizen-prompt").unwrap()).unwrap();
    let mut seen = false;
    loop {
        match table.advance(&[PlayerId(0), PlayerId(1), PlayerId(2)]).unwrap() {
            Advance::Ask(expected) => {
                if let RequestKind::ChoosePlayer { source, candidates, verb, amount, subject } = &expected.request.kind {
                    assert!(!seen);
                    assert_eq!(table.game().name(*source), "Infernal Denizen");
                    assert_eq!(expected.request.player, PlayerId(0));
                    assert_eq!(candidates, &[PlayerId(1), PlayerId(2)]);
                    assert_eq!(*verb, "may gain control of a creature you control");
                    assert_eq!(*amount, 0);
                    assert_eq!(*subject, None);
                    seen = true;
                }
                table.answered(expected.intent).unwrap();
            }
            Advance::Check(_) => {}
            Advance::Done => break,
        }
    }
    table.verdict().unwrap();
    assert!(seen);
}

#[test]
fn text_changed_sacrifice_caption_is_truthful() {
    let text = include_str!("../../../scenarios/infernal-denizen-magical-hack-changes-upkeep.cantrip");
    let mut table = Scripted::deal(GameScript::parse(text, "infernal-denizen-hacked-prompt").unwrap()).unwrap();
    let mut seen = false;
    loop {
        match table.advance(&[PlayerId(0), PlayerId(1)]).unwrap() {
            Advance::Ask(expected) => {
                if let RequestKind::ChooseCards { source, purpose: cantrip_engine::Purpose::Sacrifice, rule, .. } = &expected.request.kind {
                    assert!(!seen);
                    assert_eq!(table.game().name(*source), "Infernal Denizen");
                    assert_eq!(rule.kind, "land", "caption must remain true after Swamp becomes Forest");
                    seen = true;
                }
                table.answered(expected.intent).unwrap();
            }
            Advance::Check(_) => {}
            Advance::Done => break,
        }
    }
    table.verdict().unwrap();
    assert!(seen);
}
