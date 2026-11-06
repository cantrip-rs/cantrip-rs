use cantrip_engine::{first_choices, PlayerId, Purpose, RequestKind};
use cantrip_script::{Advance, GameScript, Scripted};

fn choice(script: &str, expected_kind: &str, expected_card: &str) {
    let script = GameScript::parse(script, "Gaea's Touch choice").unwrap();
    let mut table = Scripted::deal(script).unwrap();
    let mut found = false;
    loop {
        match table.advance(&[PlayerId(0), PlayerId(1)]).unwrap() {
            Advance::Ask(expected) => {
                if let RequestKind::ChooseCards { purpose: Purpose::PutOntoBattlefield, rule, .. } = &expected.request.kind {
                    assert_eq!(rule.kind, expected_kind);
                    let cards = first_choices(table.game(), &expected.request);
                    assert!(cards.iter().any(|&c| table.game().name(c) == expected_card));
                    found = true;
                }
                table.answered(expected.intent).unwrap();
            }
            Advance::Check(_) => panic!("unexpected client check"),
            Advance::Done => break,
        }
    }
    assert!(found, "choice was asked");
    table.verdict().unwrap();
}

#[test]
fn choice_caption_follows_land_type_text() {
    choice(include_str!("../../../scenarios/gaeas-touch-puts-basic-forest-once-per-turn.cantrip"), "basic Forest", "Forest");
    let changed = include_str!("../../../scenarios/gaeas-touch-text-change-island.cantrip");
    for (land, kind) in [("Island", "basic Island"), ("Plains", "basic Plains"), ("Swamp", "basic Swamp"), ("Mountain", "basic Mountain")] {
        choice(&changed.replace("Island", land), kind, land);
    }
}
