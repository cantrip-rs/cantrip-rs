use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn text_changed_clay_offers_an_accurate_form_label() {
    let source = include_str!("../../../scenarios/primal-clay-text-change-before-entry.cantrip")
        .replace("Bot1.Modes(1/6 Elf artifact creature with defender)", "[Handover Bot1.Modes]");
    let source = source.split("[Expect]").next().unwrap();
    let script = GameScript::parse(source, "primal-clay-prompt").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let RequestKind::ChooseModes { modes, eligible, count, .. } = &game.pending().expect("choice").kind else { panic!("expected form choice") };
    assert_eq!(*count, 1);
    assert_eq!(eligible, &[0, 1, 2]);
    let Label::WordedOption(card, index, _) = modes[2] else { panic!("expected text-adjusted option") };
    assert_eq!(game.cards.get(card).labels[index as usize], "1/6 Wall artifact creature with defender");
    assert_eq!(label_text_in(game.cards, &modes[2]), "1/6 Elf artifact creature with defender");
}

#[test]
fn printed_clay_offers_wall_form() {
    let source = include_str!("../../../scenarios/primal-clay-wall.cantrip")
        .split("Bot1.Modes(1/6 Wall artifact creature with defender)").next().unwrap().to_owned()
        + "[Handover Bot1.Modes]\n";
    let script = GameScript::parse(&source, "primal-clay-printed-prompt").unwrap();
    let game = run(&script).map_err(|f| f.report).unwrap().game;
    let RequestKind::ChooseModes { modes, eligible, count, .. } = &game.pending().expect("choice").kind else { panic!("expected form choice") };
    assert_eq!(*count, 1);
    assert_eq!(eligible, &[0, 1, 2]);
    assert_eq!(label_text_in(game.cards, &modes[2]), "1/6 Wall artifact creature with defender");
}
