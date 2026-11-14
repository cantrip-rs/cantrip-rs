//! A copied card temporarily in a graveyard is not a creature card (CR 704.5e).
use cantrip_engine::*;

#[test]
fn copy_in_graveyard_is_not_a_creature_card_target() {
    let table = cantrip_engine::cards();
    let mut game = Game::new(table, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    let player = PlayerId(0);
    let creature = table.find("Grizzly Bears").unwrap();
    let normal = game.state.push_obj(Obj::new(creature, player, Zone::graveyard(player)));
    game.state.zone_push(Zone::graveyard(player), normal);
    let mut copy = Obj::new(creature, player, Zone::graveyard(player));
    copy.card_copy = true;
    let copied = game.state.push_obj(copy);
    game.state.zone_push(Zone::graveyard(player), copied);
    let spell = table.get(table.find("Soul Exchange").unwrap());
    let asker = Asker { source: normal, controller: player, by: None, spell: true, instance: None };
    let filter = spell.spell.unwrap().targets[0].filter;
    assert!(filter(&game, &asker, Target::Object(normal)));
    assert!(!filter(&game, &asker, Target::Object(copied)));
}
