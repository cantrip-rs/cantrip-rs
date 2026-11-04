//! The controller chooses an opponent as the delayed ability resolves, and its prompt explains the choice.
use cantrip_engine::{PlayerId, RequestKind};
use cantrip_script::{Advance, GameScript, Scripted};

#[test]
fn opponent_choice_identifies_the_land_and_its_effect() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/rainbow-vale-chooses-opponent-when-trigger-resolves.cantrip");
    let mut table = Scripted::deal(GameScript::load(&path).unwrap()).unwrap();
    let mut seen = false;
    loop {
        match table.advance(&[PlayerId(0), PlayerId(1), PlayerId(2)]).unwrap() {
            Advance::Ask(expected) => {
                if let RequestKind::ChoosePlayer { source, candidates, verb, amount, subject } = &expected.request.kind {
                    assert!(!seen);
                    assert_eq!(expected.request.player, PlayerId(0));
                    assert_eq!(table.game().name(*source), "Rainbow Vale");
                    assert_eq!(candidates, &vec![PlayerId(1), PlayerId(2)]);
                    assert_eq!(*verb, "gains control of this land");
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
    assert!(seen, "the opponent choice must be asked");
}
