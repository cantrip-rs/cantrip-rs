//! Rohgahh's opponent choice has descriptive words and no numeric amount.
use cantrip_engine::{PlayerId, RequestKind};
use cantrip_script::{Advance, GameScript, Scripted};

#[test]
fn opponent_choice_describes_control_without_a_number() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/rohgahh-of-kher-keep-declined-upkeep-gives-control.cantrip");
    let mut table = Scripted::deal(GameScript::load(&path).unwrap()).unwrap();
    let mut seen = false;
    loop {
        match table.advance(&[PlayerId(0), PlayerId(1)]).unwrap() {
            Advance::Ask(expected) => {
                if let RequestKind::ChoosePlayer { source, candidates, verb, amount, subject } = &expected.request.kind {
                    assert!(!seen);
                    assert_eq!(table.game().name(*source), "Rohgahh of Kher Keep");
                    assert_eq!(expected.request.player, PlayerId(0));
                    assert_eq!(candidates, &vec![PlayerId(1)]);
                    assert_eq!(*verb, "gains control of those permanents");
                    assert_eq!(*amount, 0, "this choice has no numeric amount");
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
    assert!(seen, "the opponent choice must be asked");
}
