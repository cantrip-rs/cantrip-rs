//! Printed hybrid and kicker consumers of an announced X (CR 601.2b, 601.2f).
use cantrip_engine::*;
use cantrip_state::{Frame, FrameKind};
const YOU: PlayerId = PlayerId(0);
fn flood_lands(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    for land in g.permanents_of(item.controller) {
        if g.chars(land).is(CardType::Land) { g.add_counters(land, CounterKind::Flood, 1); }
    }
    Ok(())
}
static FLOODER: CardDef = CardDef {
    name: "Flood counter setup", oracle: true, types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost::NONE, flood_lands, "Put a flood counter on each land you control.")],
    ..CardDef::BLANK
};
fn put_eluge_and_flooded_island(g: &mut Game) {
    let all = cantrip_cards::all();
    let eluge = g.cards.id_of(all.iter().copied().find(|c| c.name == "Eluge, the Shoreless Sea").unwrap());
    let island = g.cards.id_of(all.iter().copied().find(|c| c.name == "Island").unwrap());
    let zone = Zone::BATTLEFIELD;
    let reducer = g.state.push_obj(Obj::new(eluge, YOU, zone));
    g.state.zone_push(zone, reducer);
    let land = g.state.push_obj(Obj::new(island, YOU, zone));
    g.state.zone_push(zone, land);
    g.state.update_obj(land, |o| o.tapped = true);
    let flooder = g.state.push_obj(Obj::new(g.cards.id_of(&FLOODER), YOU, zone));
    g.state.zone_push(zone, flooder);
    g.run();
    let request = g.pending().cloned().unwrap();
    let RequestKind::Priority { options } = &request.kind else { panic!("priority") };
    let option = options.iter().find(|o| o.source == Some(flooder)).unwrap();
    g.answer(request.id, YOU, Answer::Act(Action::of(option))).unwrap();
    for _ in 0..2 {
        g.run();
        let request = g.pending().cloned().unwrap();
        g.answer(request.id, request.player, Answer::Pass).unwrap();
    }
    g.run();
    assert_eq!(g.state.obj(land).counters(CounterKind::Flood), 1);
}
fn setup(name: &str) -> (Game, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&FLOODER]))));
    let all = cantrip_cards::all();
    let island = table.id_of(all.iter().copied().find(|c| c.name == "Island").unwrap());
    let card = table.id_of(all.iter().copied().find(|c| c.name == name).unwrap());
    let mut g = Game::new(table, &Setup { decks: vec![vec![island; 20], vec![island; 20]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    let zone = Zone::hand(YOU);
    let source = g.state.push_obj(Obj::new(card, YOU, zone));
    g.state.zone_push(zone, source);
    (g, source)
}
fn begin(g: &mut Game, source: ObjectId, n: u32) {
    g.run();
    let request = g.pending().cloned().unwrap();
    let RequestKind::Priority { options } = &request.kind else { panic!("priority: {:?}", request.kind) };
    let option = options.iter().find(|o| o.source == Some(source)).unwrap();
    g.answer(request.id, YOU, Answer::Act(Action::of(option))).unwrap();
    g.run();
    let request = g.pending().cloned().unwrap();
    assert!(matches!(request.kind, RequestKind::PayCosts { announcing: true, .. }), "{:?}", request.kind);
    g.answer(request.id, YOU, Answer::Announce(n)).unwrap();
    g.run();
}
#[test]
fn biomass_mutation_hybrid_reduction_keeps_announced_x() {
    let (mut g, source) = setup("Biomass Mutation");
    g.state.add_mana(YOU, Mana::C, 1001, ObjectId::NONE, None, ManaLasts::Step, false);
    g.state.add_mana(YOU, Mana::G, 1, ObjectId::NONE, None, ManaLasts::Step, false);
    put_eluge_and_flooded_island(&mut g);
    begin(&mut g, source, 1001);
    let request = g.pending().cloned().unwrap();
    let RequestKind::ChooseModes { additional_cost: true, affordable, .. } = &request.kind else { panic!("{:?}", request.kind) };
    assert_eq!(affordable, &[0], "with enough mana, advice selects one green half");
    g.answer(request.id, YOU, Answer::Modes(vec![1])).unwrap();
    g.run();
    assert_eq!(g.state.stack.last().expect("spell on stack").x, 1001);
    assert_eq!(g.state.player(YOU).pool.get(Mana::C), 0, "pending: {:?}", g.pending().map(|p| &p.kind));
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 0);
}
#[test]
fn thieving_skydiver_kicker_keeps_x_above_old_cap() {
    let (mut g, source) = setup("Thieving Skydiver");
    g.state.add_mana(YOU, Mana::C, 1002, ObjectId::NONE, None, ManaLasts::Step, false);
    g.state.add_mana(YOU, Mana::U, 1, ObjectId::NONE, None, ManaLasts::Step, false);
    begin(&mut g, source, 1001);
    assert_eq!(g.state.stack.last().expect("spell on stack").x, 1001);
    assert_eq!(g.state.stack.last().expect("spell on stack").kicked, 1);
    assert_eq!(g.state.player(YOU).pool.get(Mana::C), 0);
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 0);
}

#[test]
fn biomass_hybrid_advice_counts_the_announced_generic_mana() {
    let (mut g, source) = setup("Biomass Mutation");
    g.state.add_mana(YOU, Mana::C, 1000, ObjectId::NONE, None, ManaLasts::Step, false);
    g.state.add_mana(YOU, Mana::G, 1, ObjectId::NONE, None, ManaLasts::Step, false);
    put_eluge_and_flooded_island(&mut g);
    begin(&mut g, source, 1001);
    let request = g.pending().cloned().unwrap();
    let RequestKind::ChooseModes { additional_cost: true, affordable, .. } = &request.kind else { panic!("{:?}", request.kind) };
    assert!(affordable.is_empty(), "the announced X leaves every hybrid choice short of mana");
}
