//! Dreams targets cards, not copies of cards, even if a copy is in a graveyard.
use cantrip_engine::*;
use cantrip_state::Obj;

#[test]
fn target_filter_distinguishes_card_from_card_copy() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let you = PlayerId(0);
    let filler = table.find("Island").unwrap();
    let mut game = Game::new(table, &Setup { decks: vec![vec![filler; 30]; 2], ..Default::default() });
    let dreams = table.find("Dreams of the Dead").unwrap();
    let lion = table.find("Savannah Lions").unwrap();
    let source = game.state.push_obj(Obj::new(dreams, you, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let graveyard = Zone::graveyard(you);
    let card = game.state.push_obj(Obj::new(lion, you, graveyard));
    game.state.zone_push(graveyard, card);
    let mut copy = Obj::new(lion, you, graveyard);
    copy.card_copy = true;
    let copy = game.state.push_obj(copy);
    game.state.zone_push(graveyard, copy);
    let AbilityDef::Activated(ability) = table.get(dreams).abilities[0] else { panic!("expected activation") };
    let filter = ability.targets[0].filter;
    let asker = Asker::new(source, you);
    assert!(filter(&game, &asker, Target::Object(card)));
    assert!(!filter(&game, &asker, Target::Object(copy)));
}
