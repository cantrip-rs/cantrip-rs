use cantrip_engine::*;

#[test]
fn chasm_bars_its_controllers_creatures_only() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let card = |name| table.find(name).unwrap();
    let deck = vec![card("Forest"); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    let you = PlayerId(0);
    let them = PlayerId(1);
    let mut permanent = |name, owner| {
        let id = game.state.push_obj(Obj::new(card(name), owner, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    permanent("Glacial Chasm", you);
    let yours = permanent("Grizzly Bears", you);
    let theirs = permanent("Grizzly Bears", them);
    game.state.see_statics();
    assert!(!game.attack_rules_allow(yours));
    assert!(game.attack_rules_allow(theirs));
}
