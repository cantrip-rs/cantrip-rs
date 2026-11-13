use std::collections::HashSet;
use cantrip_engine::{visibility::observable, *};
use cantrip_script::{run, GameScript};

#[test]
fn both_hands_are_visible_only_after_revelation_resolves() {
    let source = include_str!("../../../scenarios/revelation-reveals-both-players-hands.cantrip");
    let prefix = source.split("Bot1.Hand.Forest.Play").next().unwrap();
    let before = run(&GameScript::parse(prefix, "revelation-before").unwrap()).unwrap_or_else(|f| panic!("{}", f.report)).game;
    for (viewer, other) in [(PlayerId(0), PlayerId(1)), (PlayerId(1), PlayerId(0))] {
        let visible = observable(&before, viewer, &HashSet::new());
        assert!(before.state.zone(Zone::hand(other)).iter().all(|card| !visible.contains(card)));
    }
    let game = run(&GameScript::parse(source, "revelation-visible").unwrap()).unwrap_or_else(|f| panic!("{}", f.report)).game;
    let revelation = game.state.battlefield().iter().copied().find(|&id| game.card(game.obj(id).card).name == "Revelation").unwrap();
    assert!(game.card(game.obj(revelation).card).supertypes.world);
    for (viewer, other, name, count) in [(PlayerId(0), PlayerId(1), "Island", 7), (PlayerId(1), PlayerId(0), "Forest", 5)] {
        let visible = observable(&game, viewer, &HashSet::new());
        let hand = game.state.zone(Zone::hand(other));
        assert_eq!(hand.len(), count);
        assert!(hand.iter().all(|&card| visible.contains(&card) && game.card(game.obj(card).card).name == name));
    }
}
