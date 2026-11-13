//! The granted ability's text identifies the quality for players inspecting the creature.
use cantrip_engine::*;
use std::sync::OnceLock;

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all()))
}

#[test]
fn granted_banding_names_legendary_creatures() {
    let mut game = Game::new(table(), &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let put = |game: &mut Game, name: &str| {
        let card = table().find(name).unwrap();
        let id = game.state.push_obj(Obj::new(card, PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        game.state.see_statics();
        id
    };
    put(&mut game, "Unholy Citadel");
    let creature = put(&mut game, "Arnyn, Deathbloom Botanist");
    let abilities = game.chars(creature).abilities;
    let granted = abilities.iter().flatten().find(|&&a| matches!(table().ability(a), AbilityDef::BandsWithOthers(_))).expect("granted banding");
    assert_eq!(table().ability(*granted).text(), "Bands with other legendary creatures.");
}
