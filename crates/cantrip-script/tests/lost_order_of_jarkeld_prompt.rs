//! The entry choice describes the chosen opponent without a numeric amount.
use cantrip_engine::{PlayerId, RequestKind};
use cantrip_script::{Advance, GameScript, Scripted};

#[test]
fn entry_choice_has_accurate_metadata() {
    let text = include_str!("../../../scenarios/lost-order-of-jarkeld-chosen-opponent.cantrip");
    let mut table = Scripted::deal(GameScript::parse(text, "lost-order-choice").unwrap()).unwrap();
    let mut seen = false;
    loop {
        match table.advance(&[PlayerId(0), PlayerId(1), PlayerId(2)]).unwrap() {
            Advance::Ask(expected) => {
                if let RequestKind::ChoosePlayer { source, candidates, verb, amount, subject } = &expected.request.kind {
                    assert!(!seen);
                    assert_eq!(expected.request.player, PlayerId(0));
                    assert_eq!(table.game().name(*source), "Lost Order of Jarkeld");
                    assert_eq!(candidates, &[PlayerId(1), PlayerId(2)]);
                    assert_eq!(*verb, "has their creatures counted for this creature's power and toughness");
                    assert_eq!(*amount, 0, "no numeric amount belongs to this choice");
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
    assert!(seen, "the entering choice was asked");
}
