use cantrip_api::*;
use cantrip_engine::{Game, Setup, replaced_mana_yields};

fn you_tap(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ManaProduction(m) if m.player == asker.controller)
}
fn double(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ManaProduction(m) = proposal { m.mana = ManaPool(m.mana.0.map(|n| n * 2)); }
}
static REFLECTION: CardDef = CardDef {
    name: "Probe Mana Reflection",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ManaProduction, you_tap, double, "twice as much instead")],
    ..CardDef::BLANK
};

#[test]
fn replacement_row_includes_both_unicorn_bundles() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().iter().copied().chain([&REFLECTION]))));
    let mut game = Game::new(table, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    let player = PlayerId(0);
    let unicorn = game.state.push_obj(Obj::new(table.find("Adarkar Unicorn").unwrap(), player, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, unicorn);
    let reflection = game.state.push_obj(Obj::new(table.find("Probe Mana Reflection").unwrap(), player, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, reflection);
    game.state.see_statics();
    let rows = replaced_mana_yields(&game, player, unicorn, 0).expect("replacement changes the output");
    let outputs = &rows[0].1;
    assert_eq!(outputs.len(), 2, "the row offers exactly two alternatives: {outputs:?}");
    assert!(outputs.iter().any(|m| m.get(Mana::U) == 2 && m.get(Mana::C) == 0), "missing doubled {{U}} choice: {outputs:?}");
    assert!(outputs.iter().any(|m| m.get(Mana::U) == 2 && m.get(Mana::C) == 2), "missing doubled {{C}}{{U}} choice: {outputs:?}");
}

#[test]
fn producible_types_include_blue_and_colorless() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().iter().copied().chain([&REFLECTION]))));
    let mut game = Game::new(table, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    let player = PlayerId(0);
    let unicorn = game.state.push_obj(Obj::new(table.find("Adarkar Unicorn").unwrap(), player, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, unicorn);
    assert_eq!(game.chars(unicorn).abilities().count(), 1, "the two bundles belong to one ability");
    let (colors, colorless) = Query::producible_types(&game, unicorn);
    assert!(colors.has(Color::Blue));
    assert!(colorless);
}
