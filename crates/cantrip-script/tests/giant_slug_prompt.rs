use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn upkeep_choice_offers_five_fixed_land_types() {
    let source = include_str!("../../../scenarios/giant-slug-islandwalk-next-upkeep.cantrip")
        .split("Bot1.Modes(Island)").next().unwrap().to_owned() + "[Handover Bot1.Modes]\n";
    let script = GameScript::parse(&source, "giant-slug-prompt").unwrap();
    let game = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report)).game;
    let request = game.pending().unwrap();
    assert_eq!(request.player, PlayerId(0));
    let RequestKind::ChooseModes { modes, count, .. } = &request.kind else { panic!("expected basic land choice") };
    assert_eq!(*count, 1);
    assert_eq!(modes.len(), 5);
    for (index, mode) in modes.iter().enumerate() {
        let Label::WordedOption(card, slot, words) = mode else { panic!("expected fixed option") };
        assert_eq!(usize::from(*slot), index);
        assert!(words.is_printed());
        assert_eq!(game.cards.get(*card).labels[index], ["Plains", "Island", "Swamp", "Mountain", "Forest"][index]);
    }
}
