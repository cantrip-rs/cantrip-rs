use cantrip_engine::{label_text_in, RequestKind};
use cantrip_script::{run, GameScript};

#[test]
fn color_choice_offers_one_to_five_distinct_colors() {
    let source = include_str!("../../../scenarios/dream-coat-single-color.cantrip")
        .split("! Bot1.Modes(Count: 0)").next().unwrap().to_owned()
        + "[Handover Bot1.Modes]\n";
    let script = GameScript::parse(&source, "dream-coat-prompt").unwrap();
    let game = run(&script).map_err(|failure| failure.report).unwrap().game;
    let RequestKind::ChooseModes { modes, eligible, count, at_most, additional_cost, repeat, .. } =
        &game.pending().expect("color choice").kind else { panic!("expected color choice") };
    assert_eq!((*count, *at_most), (1, 5));
    assert_eq!(eligible, &[0, 1, 2, 3, 4]);
    assert!(!additional_cost);
    assert!(!repeat);
    assert_eq!(modes.iter().map(|m| label_text_in(game.cards, m)).collect::<Vec<_>>(),
        ["White", "Blue", "Black", "Red", "Green"]);
}
