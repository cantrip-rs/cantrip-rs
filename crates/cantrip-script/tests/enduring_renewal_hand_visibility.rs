//! Enduring Renewal exposes only its controller's hand (CR 109.5).
use cantrip_engine::*;

#[test]
fn renewal_reveals_its_controllers_hand_only() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let filler = table.find("Plains").unwrap();
    let mut game = Game::new(table, &Setup { decks: vec![vec![filler; 20], vec![filler; 20]], shuffle: false, ..Setup::default() });
    let renewal = table.find("Enduring Renewal").unwrap();
    let id = game.state.push_obj(Obj::new(renewal, PlayerId(0), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.see_statics();
    assert_eq!(game.revealed_hands(), 1);
}
