//! A printed X spell beyond the old 1000 announcement cap (CR 601.2b).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};
fn enough_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::C, 1001);
    g.end_sentence();
    g.add_mana(item.controller, Mana::R, 1);
    Ok(())
}
static MANA: CardDef = CardDef {
    name: "X mana probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], enough_mana), ..CardDef::BLANK
};
#[test]
fn fireball_uses_x_1001_with_sufficient_mana() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&MANA])));
    let script = GameScript::parse(r#"
[Life]
Bot2: 2000
[Bot1 Deck]
X mana probe
Fireball
Mountain x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.X mana probe.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Fireball.Cast(X: 1001)
Bot1.Target(Bot2)
Bot1.Pass
Bot2.Pass
[Expect]
Bot2.Life = 999
"#, "fireball-x-1001").unwrap();
    let result = run_on(&script, table).unwrap_or_else(|f| panic!("{}", f.report));
    let mut game = result.game;
    let end = game.at;
    let root = game.tree.root();
    game.goto(root);
    game.goto(end);
    assert_eq!(game.state.player(PlayerId(1)).life, 999);
    assert!(game.log.iter().any(|r| matches!(r.event, Event::Damage { amount: 1001, .. })));
}

fn reduced_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::C, 1002);
    g.end_sentence();
    g.add_mana(item.controller, Mana::R, 3);
    Ok(())
}
static REDUCED_MANA: CardDef = CardDef {
    name: "Reduced X mana probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], reduced_mana), ..CardDef::BLANK
};
#[test]
fn fireball_x_1001_pays_its_reduced_actual_cost() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&REDUCED_MANA])));
    let script = GameScript::parse(r#"
[Life]
Bot2: 2000
[Bot1 Deck]
Reduced X mana probe
The Fire Crystal
Fireball
Mountain x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Reduced X mana probe.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.The Fire Crystal.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Fireball.Cast(X: 1001)
Bot1.Target(Bot2)
Bot1.Pass
Bot2.Pass
[Expect]
Bot2.Life = 999
"#, "fireball-reduced-x-1001").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|f| f.report).unwrap_or_default());
}

static BATTERY: CardDef = CardDef {
    name: "X mana battery probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, Adds { pool: ManaPool([0, 0, 0, 0, 0, 1001]), ..add_n(Mana::C, 1) }, "{T}: Add 1001 colorless mana.")],
    ..CardDef::BLANK
};
#[test]
fn mana_abilities_remain_available_after_announcing_x_1001() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&BATTERY])));
    let script = GameScript::parse(r#"
[Life]
Bot2: 2000
[Bot1 Deck]
X mana battery probe
Mox Ruby
Fireball
Mountain x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.X mana battery probe.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Fireball.Cast(X: 1001)
Bot1.Target(Bot2)
Bot1.Battlefield.X mana battery probe.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Pass
Bot2.Pass
[Expect]
Bot2.Life = 999
"#, "fireball-x-1001-mana-window").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|f| f.report).unwrap_or_default());
}

/// Nothing bounds an X paid in mana but the total's count (CR 107.3, 601.2b): the most Fireball's X may be announced as is
/// what keeps its total, {R} and X, within the announcement's signed controls, and one past it is refused.
#[test]
fn fireballs_x_is_announced_up_to_what_its_total_holds() {
    let mut g = at_main(&[]);
    let fireball = in_hand(&mut g, "Fireball");
    let r = begin(&mut g, fireball);
    let RequestKind::PayCosts { announcing: true, most, .. } = r.kind else { panic!("X is announced, got {:?}", r.kind) };
    assert_eq!(most, i32::MAX as u32 - 1);
    assert!(g.answer(r.id, YOU, Answer::Announce(most + 1)).is_err(), "past the most is refused");
    g.answer(r.id, YOU, Answer::Announce(most)).expect("the most is offered");
}

/// Chalice of the Void's {X}{X} (CR 107.3, 601.2b): each X is two generic mana, so its X holds half as many.
#[test]
fn chalice_of_the_voids_double_x_is_announced_up_to_what_its_total_holds() {
    let mut g = at_main(&[]);
    let chalice = in_hand(&mut g, "Chalice of the Void");
    let r = begin(&mut g, chalice);
    let RequestKind::PayCosts { announcing: true, most, .. } = r.kind else { panic!("X is announced, got {:?}", r.kind) };
    assert_eq!(most, i32::MAX as u32 / 2);
    assert!(g.answer(r.id, YOU, Answer::Announce(most + 1)).is_err(), "past the most is refused");
    g.answer(r.id, YOU, Answer::Announce(most)).expect("the most is offered");
}

/// Thieving Skydiver's kicker {X} (CR 702.33a, 107.3): its X is announced up to what keeps the total, {1}{U} and X, within
/// the count.
#[test]
fn a_kicker_x_is_announced_up_to_what_its_total_holds() {
    let mut g = at_main(&[]);
    g.state.add_mana(YOU, Mana::U, 1, ObjectId::NONE, None, ManaLasts::Step, false);
    let skydiver = in_hand(&mut g, "Thieving Skydiver");
    let r = begin(&mut g, skydiver);
    let RequestKind::PayCosts { announcing: true, most, .. } = r.kind else { panic!("X is announced, got {:?}", r.kind) };
    assert_eq!(most, i32::MAX as u32 - 2);
    assert!(g.answer(r.id, YOU, Answer::Announce(most + 1)).is_err(), "past the most is refused");
    g.answer(r.id, YOU, Answer::Announce(most)).expect("the most is offered");
}

static DOUBLE_X: CardDef = CardDef {
    name: "Double X activation probe", oracle: true, types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{X}{X}{2}"), nothing, "{X}{X}{2}: Nothing.")], ..CardDef::BLANK
};
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }

/// The same for an activation's {X}{X} (CR 602.2b): each X is two generic mana, so it holds half as many, past the {2}.
#[test]
fn an_activations_double_x_is_announced_up_to_what_its_total_holds() {
    let mut g = at_main(&[&DOUBLE_X]);
    let source = g.state.push_obj(Obj::new(g.cards.id_of(&DOUBLE_X), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, source);
    let r = begin(&mut g, source);
    let RequestKind::PayCosts { announcing: true, most, .. } = r.kind else { panic!("X is announced, got {:?}", r.kind) };
    assert_eq!(most, (i32::MAX as u32 - 2) / 2);
    assert!(g.answer(r.id, YOU, Answer::Announce(most + 1)).is_err(), "past the most is refused");
}

const YOU: PlayerId = PlayerId(0);

/// Your precombat main phase, nothing on the battlefield and {R}{C}{C} in your pool, `extra` test cards in the table.
fn at_main(extra: &[&'static CardDef]) -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain(extra.iter().copied()))));
    let island = table.id_of(cantrip_cards::all().into_iter().find(|c| c.name == "Island").unwrap());
    let mut g = Game::new(table, &Setup { decks: vec![vec![island; 20], vec![island; 20]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    g.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g.state.add_mana(YOU, Mana::R, 1, ObjectId::NONE, None, ManaLasts::Step, false);
    g.state.add_mana(YOU, Mana::C, 2, ObjectId::NONE, None, ManaLasts::Step, false);
    g
}

fn in_hand(g: &mut Game, name: &str) -> ObjectId {
    let card = g.cards.id_of(cantrip_cards::all().into_iter().find(|c| c.name == name).unwrap());
    let id = g.state.push_obj(Obj::new(card, YOU, Zone::hand(YOU)));
    g.state.zone_push(Zone::hand(YOU), id);
    id
}

/// Takes `source`'s action from priority: the announcement it asks first.
fn begin(g: &mut Game, source: ObjectId) -> Request {
    g.run();
    let r = g.pending().cloned().unwrap();
    let RequestKind::Priority { options } = &r.kind else { panic!("priority: {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(source)).expect("offered");
    g.answer(r.id, YOU, Answer::Act(Action::of(option))).unwrap();
    g.run();
    g.pending().cloned().unwrap()
}
