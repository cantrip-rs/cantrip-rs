use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn revealed_card_is_available_before_payment() {
    let source = include_str!("../../../scenarios/zurs-weirding-draw-payment.cantrip")
        .split("Bot1.Accept").next().unwrap().to_owned() + "[Handover Bot1.Accept]\n";
    let game = run(&GameScript::parse(&source, "weirding reveal").unwrap()).map_err(|f| f.report).unwrap().game;
    let top = game.state.zone(Zone::library(PlayerId(1)))[0];
    let library = game.state.zone(Zone::library(PlayerId(1)));
    let revealed: Vec<_> = game.log.iter().filter_map(|r| match r.event {
        Event::Revealed { card, .. } if library.contains(&card) => Some(card),
        _ => None,
    }).collect();
    assert_eq!(revealed, [top], "only the top card is revealed before payment");
    assert_eq!(game.revealed_hands(), 3);
    let RequestKind::UnlessPays { question, life, .. } = &game.pending().unwrap().kind else { panic!("expected payment choice") };
    assert_eq!(*life, 2);
    assert_eq!(question.ask, "pay {cost} to put the revealed card into its owner’s graveyard?");
    assert_eq!(question.detail, "If no player pays, that player draws a card");
}

#[test]
fn revealed_draw_remains_public_for_each_multiplayer_payer_and_undo() {
    use std::collections::HashSet;
    let original = include_str!("../../../scenarios/zurs-weirding-draw-payment.cantrip")
        .replace("[Actions]", "[Bot3 Deck]\nIsland x20\n[Actions]");
    let source = original.split("Bot1.Accept").next().unwrap().to_owned() + "[Handover Bot3.Decline]\n";
    let mut game = run(&GameScript::parse(&source, "weirding multiplayer reveal").unwrap()).map_err(|f| f.report).unwrap().game;
    let top = game.state.zone(Zone::library(PlayerId(1)))[0];
    let before = game.at;
    let first = game.pending().cloned().expect("first other player chooses");
    assert_eq!(first.player, PlayerId(2));
    for seat in 0..3 { assert!(cantrip_engine::visibility::observable(&game, PlayerId(seat), &HashSet::new()).contains(&top), "revealed card is public at the first decision"); }
    game.answer(first.id, first.player, Answer::Act(Action::of(&ActionOption::simple(ActionKind::DeclineCost)))).unwrap();
    game.run();
    let second = game.pending().cloned().expect("second other player chooses");
    assert_eq!(second.player, PlayerId(0));
    for seat in 0..3 { assert!(cantrip_engine::visibility::observable(&game, PlayerId(seat), &HashSet::new()).contains(&top), "reveal survives the prior player's decline"); }
    let after = game.at;
    game.goto(before);
    assert!(game.revealed_in_hidden_zones().contains(&top));
    game.goto(after);
    assert!(game.revealed_in_hidden_zones().contains(&top));
    let choice = game.pending().cloned().unwrap();
    game.answer(choice.id, choice.player, Answer::Act(Action::of(&ActionOption::simple(ActionKind::PayCost)))).unwrap();
    game.run();
    assert!(!game.revealed_in_hidden_zones().contains(&top), "the old library object is no longer revealed after it moves");
    assert_eq!(game.state.obj(top).became.map(|id| game.state.obj(id).zone), Some(Zone::graveyard(PlayerId(1))));
}

#[test]
fn abundance_reordering_a_weirding_revealed_card_renews_its_identity() {
    let source = r#"# Abundance replaces Weirding's subsequent draw and reorders the revealed top, which becomes a new object (CR 701.20d).
[Bot1 Deck]
Zur's Weirding
Abundance
Black Lotus x3
Mox Sapphire
Ancestral Recall
Forest
Plains
Ornithopter
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Black Lotus#1.Cast
Bot1.Hand.Black Lotus#2.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Battlefield.Black Lotus#1.Activate(Color: Green)
Bot1.Battlefield.Black Lotus#2.Activate(Color: Green)
Bot1.Hand.Abundance.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Hand.Zur's Weirding.Cast
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Ancestral Recall.Cast(Bot1)
Bot1.Replace(Zur's Weirding)
Bot2.Decline
Bot1.Accept
Bot1.Modes(Nonland)
[Handover Bot1.Arrange]
"#;
    let mut game = run(&GameScript::parse(source, "weirding abundance").unwrap()).map_err(|f| f.report).unwrap().game;
    let request = game.pending().cloned().expect("Abundance asks to arrange the remaining cards");
    let RequestKind::DistributeCards { cards, .. } = &request.kind else { panic!("expected library arrangement") };
    let old = cards[0];
    assert!(game.revealed_in_hidden_zones().contains(&old));
    let placements = vec![cards.clone()];
    game.answer(request.id, request.player, Answer::Placements { placements, shuffle: false }).unwrap();
    game.run();
    assert_eq!(game.state.obj(old).zone, Zone::GONE, "CR 701.20d renews the reordered revealed object");
    assert_eq!(game.state.obj(old).became, None);
}
