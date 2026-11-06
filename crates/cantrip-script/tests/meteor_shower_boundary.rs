//! Meteor Shower's announced division total at the current upstream u32 boundary (RX-5).

use cantrip_engine::*;

#[test]
fn meteor_shower_division_total_saturates_at_maximum_x() {
    let card = cantrip_cards::all().into_iter().find(|card| card.name == "Meteor Shower").unwrap();
    let division = card.divide.expect("Meteor Shower divides its damage");
    let table = cards();
    let island = table.find("Island").unwrap();
    let game = Game::new(table, &Setup { decks: vec![vec![island; 30]; 2], ..Default::default() });
    let asker = Asker::new(ObjectId(0), PlayerId(0));
    assert_eq!(division.total_for(&game, &asker, u32::MAX - 1), u32::MAX);
    assert_eq!(division.total_for(&game, &asker, u32::MAX), u32::MAX);
}
