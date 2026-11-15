//! "Spend only black mana on X" (CR 107.3, 601.2f-h), over test cards: X's generic mana is paid only by the mana the card
//! allows (`XMana`, `ManaCost::x_part`), the rest of the cost by any. The payment refuses other mana for X; mana that
//! could pay either is split between them (`ways`, the payment's own spending, the advice `can_afford`); the most X the
//! mana at hand could pay for comes from that split (`most_affordable_x`); a generic reduction may take X's part (Drain
//! Life's ruling), a cost increase is not X's (Atalya's ruling); and "Spend only colored mana on X. No more than one mana
//! of each color may be spent this way" (Emblazoned Golem) takes one of each color at most.

use crate::actions::{can_afford, most_affordable_x, ways, Ways};
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static SWAMP: CardDef = CardDef {
    name: "X Mana Test Swamp",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add(Mana::B), "{T}: Add {B}.")],
    ..CardDef::BLANK
};

static MOUNTAIN: CardDef = CardDef {
    name: "X Mana Test Mountain",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add(Mana::R), "{T}: Add {R}.")],
    ..CardDef::BLANK
};

/// Drain Life's cost and its restriction: "{X}{1}{B} ... Spend only black mana on X."
static DRAIN: CardDef = CardDef {
    name: "X Mana Test Drain",
    cost: mana("{X}{1}{B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], nothing),
    x_mana: XMana::only(Mana::B),
    ..CardDef::BLANK
};

/// Crypt Rats's ability: "{X}: ... Spend only black mana on X."
static RATS: CardDef = CardDef {
    name: "X Mana Test Rats",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(of_mana(mana("{X}")), nothing, "{X}: Nothing. Spend only black mana on X.").x_mana(XMana::only(Mana::B))],
    ..CardDef::BLANK
};

/// Emblazoned Golem: "{2}. Kicker {X}. Spend only colored mana on X. No more than one mana of each color may be spent this
/// way."
static GOLEM: CardDef = CardDef {
    name: "X Mana Test Golem",
    cost: mana("{2}"),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(2),
    kicker: kicker(mana("{X}")),
    x_mana: XMana::COLORED_ONE_EACH,
    ..CardDef::BLANK
};

/// "Sorcery spells you cast cost {1} less to cast."
fn sorceries_cost_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(spell) if player == q.controller(source) && q.chars(spell).is(CardType::Sorcery) => CostDelta { less: mana("{1}"), ..CostDelta::NONE },
        _ => CostDelta::NONE,
    }
}

static REDUCER: CardDef = CardDef { name: "X Mana Test Reducer", types: TypeSet::of(&[CardType::Artifact]), cost_changes: &[sorceries_cost_less], ..CardDef::BLANK };

/// "Sorcery spells you cast cost {1} more to cast."
fn sorceries_cost_more(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    match costed {
        Costed::Spell(spell) if player == q.controller(source) && q.chars(spell).is(CardType::Sorcery) => CostDelta::more(1),
        _ => CostDelta::NONE,
    }
}

static TAXER: CardDef = CardDef { name: "X Mana Test Taxer", types: TypeSet::of(&[CardType::Artifact]), cost_changes: &[sorceries_cost_more], ..CardDef::BLANK };

fn enchantment_card(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Enchantment)
}

/// Drain with the Marches' "As an additional cost to cast this spell, you may exile any number of [enchantment] cards from
/// your hand. This spell costs {2} less to cast for each card exiled this way."
static MARCH: CardDef = CardDef {
    name: "X Mana Test March",
    additional_cost: Some(Cost { exile_from_hand_for_less: Some(ExileFromHandForLess { filter: enchantment_card, less_each: 2 }), ..Cost::NONE }),
    ..DRAIN
};

static FODDER: CardDef = CardDef { name: "X Mana Test Fodder", types: TypeSet::of(&[CardType::Enchantment]), ..CardDef::BLANK };

/// An artifact with a kicker that is not X ("Kicker {1}") and an ability with {X}: the ability's X is no kicker's.
static KICKED_RATS: CardDef = CardDef {
    name: "X Mana Test Kicked Rats",
    cost: mana("{2}"),
    types: TypeSet::of(&[CardType::Artifact]),
    kicker: kicker(mana("{1}")),
    abilities: &[ability(of_mana(mana("{X}")), nothing, "{X}: Nothing.")],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef {
    name: "X Mana Test Bear",
    cost: mana("{1}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield) && q.chars(id).is(CardType::Creature))
}

/// "{X}{B} sorcery: target creature. Spend only black mana on X.", its target chosen once X is announced (CR 601.2c).
static SAP: CardDef = CardDef {
    name: "X Mana Test Sap",
    cost: mana("{X}{B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[target(creature)], nothing),
    late_targets: true,
    x_mana: XMana::only(Mana::B),
    ..CardDef::BLANK
};

/// "Spells that target a creature cost {2} more to cast": an increase that waits on the targets (CR 601.2c, 601.2f).
fn targeting_a_creature_costs_more(q: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, targets: &[Target]) -> CostDelta {
    let creature = targets.iter().any(|&t| matches!(t, Target::Object(id) if q.chars(id).is(CardType::Creature)));
    match costed {
        Costed::Spell(_) if creature => CostDelta::more(2),
        _ => CostDelta::NONE,
    }
}

static WARDEN: CardDef = CardDef { name: "X Mana Test Warden", types: TypeSet::of(&[CardType::Artifact]), cost_changes: &[targeting_a_creature_costs_more], ..CardDef::BLANK };

/// Your precombat main phase with priority, nothing anywhere.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &SWAMP, &MOUNTAIN, &DRAIN, &RATS, &GOLEM, &REDUCER, &TAXER, &MARCH, &FODDER, &KICKED_RATS, &BEAR, &SAP, &WARDEN,
    ])));
    let deck = || vec![table.id_of(&SWAMP); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

fn add_mana(g: &mut Game, pool: &[Mana]) {
    for &m in pool {
        g.state.add_mana(YOU, m, 1, ObjectId::NONE, None, ManaLasts::Step, false);
    }
}

fn pool(of: &[Mana]) -> ManaPool {
    let mut p = ManaPool::default();
    for &m in of {
        p.0[m as usize] += 1;
    }
    p
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

/// Takes the first option `source` has at priority, and gives back the announcement of X it asks.
fn announce(g: &mut Game, source: ObjectId) -> Request {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(source) && o.index == 0).expect("it is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    let r = next(g);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, .. }), "expected the announcement of X, got {:?}", r.kind);
    r
}

/// Announces X as `x` and gives back the payment it asks.
fn announce_as(g: &mut Game, r: &Request, x: u32) -> Request {
    let r = announce_paid(g, r, x);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: false, .. }), "expected the payment, got {:?}", r.kind);
    r
}

/// Announces X as `x` and gives back what is asked next: priority, where the pool paid the cost in its one way.
fn announce_paid(g: &mut Game, r: &Request, x: u32) -> Request {
    g.answer(r.id, r.player, Answer::Announce(x)).expect("a legal announcement");
    next(g)
}

/// The mana the payment offers to spend from the pool.
fn spendable(r: &Request) -> Vec<Mana> {
    let RequestKind::PayCosts { options, .. } = &r.kind else { panic!("expected a payment, got {:?}", r.kind) };
    options.iter().filter(|o| o.kind == ActionKind::AssignMana).filter_map(|o| o.mana).collect()
}

/// Spends one `mana` from the pool, and gives back what is asked next.
fn spend(g: &mut Game, r: &Request, mana: Mana) -> Request {
    let RequestKind::PayCosts { options, .. } = &r.kind else { panic!("expected a payment, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.kind == ActionKind::AssignMana && o.mana == Some(mana)).unwrap_or_else(|| panic!("{mana:?} is not offered")).clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    next(g)
}

/// The most X the announcement `r` says the mana at hand could pay for (`most_affordable_x`), as the host sends it.
fn most_x(g: &Game, r: &Request) -> Option<u32> {
    let RequestKind::PayCosts { source, x_cost, least, most, .. } = &r.kind else { panic!("expected the announcement, got {:?}", r.kind) };
    most_affordable_x(g, YOU, *source, x_cost.as_deref().expect("an X paid in mana"), *least, *most)
}

/// Drain's cost with X announced as `x`.
fn drain_at(x: u32) -> ManaCost {
    mana("{1}{B}").with_x_mana(XMana::only(Mana::B)).plus_x(x, XMana::only(Mana::B))
}

#[test]
fn x_part_is_the_announced_x_and_an_increase_is_not_x() {
    let mut cost = drain_at(2);
    assert_eq!((cost.generic, cost.x_part(), cost.plain_generic()), (3, 2, 1));
    cost.add_generic(1);
    assert_eq!((cost.generic, cost.x_part()), (4, 2), "a cost increase is not X's (Atalya's ruling)");
    cost.generic -= 1;
    assert_eq!(cost.x_part(), 1, "a generic reduction takes X's part first (Drain Life's ruling)");
    assert_eq!(mana("{1}{B}").with_x_mana(XMana::only(Mana::B)).x_part(), 0, "no X announced, no X's part");
}

#[test]
fn black_only_x_is_refused_other_mana() {
    // {X}{1}{B} with X 2: four mana, three of them red - enough mana, but X's two may only be black.
    assert!(matches!(ways(&pool(&[Mana::B, Mana::R, Mana::R, Mana::R]), &drain_at(2)), Ways::None));
    assert!(matches!(ways(&pool(&[Mana::B, Mana::R, Mana::R, Mana::R]), &drain_at(0)), Ways::Many | Ways::One(_)));

    let mut g = game();
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::B, Mana::R, Mana::R, Mana::R]);
    let r = announce(&mut g, drain);
    let r = announce_as(&mut g, &r, 2);
    let r = spend(&mut g, &r, Mana::B);
    let r = spend(&mut g, &r, Mana::R);
    assert!(spendable(&r).is_empty(), "with only X's {{2}} owed, red mana is not offered for it: {:?}", spendable(&r));
}

#[test]
fn black_only_x_of_an_ability_is_refused_other_mana() {
    let mut g = game();
    let rats = put(&mut g, &RATS, Zone::BATTLEFIELD);
    add_mana(&mut g, &[Mana::R, Mana::B]);
    let r = announce(&mut g, rats);
    let r = announce_as(&mut g, &r, 2);
    assert_eq!(spendable(&r), vec![Mana::B], "only the black mana pays the ability's X");
    let r = spend(&mut g, &r, Mana::B);
    assert!(spendable(&r).is_empty(), "the red can't pay the second");
}

#[test]
fn a_pool_of_black_and_other_mana_is_split_between_x_and_the_rest() {
    // {X}{1}{B} with X 1 from {B}{B}{R}: the black pays {B} and X, the red the {1} - one way only.
    assert!(matches!(ways(&pool(&[Mana::B, Mana::B, Mana::R]), &drain_at(1)), Ways::One([0, 0, 2, 1, 0, 0])));

    let mut g = game();
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::B, Mana::B, Mana::R]);
    let r = announce(&mut g, drain);
    // The pool's one way pays it at once.
    let r = announce_paid(&mut g, &r, 1);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "paid without a question, got {:?}", r.kind);
    assert!(g.state.player(YOU).pool.is_empty(), "the pool paid it all");
    assert_eq!(g.state.stack.len(), 1, "Drain is cast");
}

#[test]
fn spending_a_black_on_the_generic_mana_pays_x_first() {
    let mut g = game();
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::B, Mana::B, Mana::B, Mana::R, Mana::G]);
    let r = announce(&mut g, drain);
    let r = announce_as(&mut g, &r, 2);
    let r = spend(&mut g, &r, Mana::B);
    let r = spend(&mut g, &r, Mana::B);
    let r = spend(&mut g, &r, Mana::B);
    // The two black after the {B} paid X's {2}: the red or the green pays the {1}.
    assert_eq!(spendable(&r), vec![Mana::R, Mana::G]);
    spend(&mut g, &r, Mana::R);
    assert_eq!(g.state.stack.len(), 1, "Drain is cast");
}

#[test]
fn the_advice_splits_the_sources_between_x_and_the_rest() {
    let mut g = game();
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    for def in [&SWAMP, &SWAMP, &SWAMP, &MOUNTAIN, &MOUNTAIN] {
        put(&mut g, def, Zone::BATTLEFIELD);
    }
    let c = cx(&mut g);
    // Three Swamps and two Mountains: {B}, X's black, and the {1} from a Mountain.
    assert!(can_afford(&c, YOU, drain, &drain_at(2)));
    assert!(!can_afford(&c, YOU, drain, &drain_at(3)), "five mana, but only two Swamps left for X");
    assert!(can_afford(&c, YOU, drain, &mana("{4}{B}")), "unrestricted, the same mana pays {{4}}{{B}}");
}

#[test]
fn the_most_x_offered_comes_from_the_split() {
    let mut g = game();
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    for def in [&SWAMP, &SWAMP, &SWAMP, &MOUNTAIN, &MOUNTAIN] {
        put(&mut g, def, Zone::BATTLEFIELD);
    }
    let r = announce(&mut g, drain);
    assert_eq!(most_x(&g, &r), Some(2), "X up to the Swamps the {{B}} leaves, not five mana's 3");

    // No Swamp: X 0 only.
    let mut g = game();
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    put(&mut g, &SWAMP, Zone::BATTLEFIELD);
    for _ in 0..4 {
        put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    }
    let r = announce(&mut g, drain);
    assert_eq!(most_x(&g, &r), Some(0));
}

#[test]
fn a_reduction_may_take_x_and_other_mana_still_cant_pay_the_rest_of_x() {
    let mut g = game();
    put(&mut g, &REDUCER, Zone::BATTLEFIELD);
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    // X 2: {2}{1}{B} less {1} is {2}{B}, of which X's is {1} (the reduction took X's first): {B}{B}{R} pays it, and a
    // green as well as the red could pay the {1}.
    add_mana(&mut g, &[Mana::B, Mana::B, Mana::R, Mana::G]);
    let r = announce(&mut g, drain);
    let r = announce_as(&mut g, &r, 2);
    let RequestKind::PayCosts { remaining, .. } = &r.kind else { unreachable!() };
    assert_eq!((remaining.generic, remaining.x_part()), (2, 1));
    let r = spend(&mut g, &r, Mana::B);
    spend(&mut g, &r, Mana::R);
    // The red paid the {1}; what is left of X is black only, so the pool's one way pays it with the black, not the green.
    assert_eq!(g.state.stack.len(), 1, "Drain is cast");
    assert_eq!(g.state.player(YOU).pool, pool(&[Mana::G]));

    // With {B}{R}{R}, the reduced X's {1} has no black to pay it.
    let mut g = game();
    put(&mut g, &REDUCER, Zone::BATTLEFIELD);
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::B, Mana::R, Mana::R]);
    let r = announce(&mut g, drain);
    let r = announce_as(&mut g, &r, 2);
    let r = spend(&mut g, &r, Mana::B);
    let r = spend(&mut g, &r, Mana::R);
    assert!(spendable(&r).is_empty(), "red can't pay what the reduction left of X");
}

#[test]
fn a_cost_increase_is_paid_with_any_mana() {
    let mut g = game();
    put(&mut g, &TAXER, Zone::BATTLEFIELD);
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    // X 1: {1}{1}{B} and {1} more: X's {1} black, the {2} red.
    add_mana(&mut g, &[Mana::B, Mana::B, Mana::R, Mana::R, Mana::G]);
    let r = announce(&mut g, drain);
    let r = announce_as(&mut g, &r, 1);
    let RequestKind::PayCosts { remaining, .. } = &r.kind else { unreachable!() };
    assert_eq!((remaining.generic, remaining.x_part()), (3, 1));
    assert!(matches!(ways(&pool(&[Mana::B, Mana::B, Mana::R, Mana::R]), remaining), Ways::One([0, 0, 2, 2, 0, 0])));
    assert!(matches!(ways(&pool(&[Mana::B, Mana::R, Mana::R, Mana::R]), remaining), Ways::None), "the increase is not X's, but X's {{1}} still is");
}

#[test]
fn colored_x_takes_one_mana_of_each_color_at_most() {
    let golem = |x: u32| mana("{2}").with_x_mana(XMana::COLORED_ONE_EACH).plus_x(x, XMana::COLORED_ONE_EACH);
    assert_eq!(golem(2).x_shapes().len(), 10, "two colors of five");
    assert!(golem(6).x_shapes().is_empty(), "six colors there are not");
    // Two white can't pay X 2: one of each color.
    assert!(matches!(ways(&pool(&[Mana::W, Mana::W, Mana::C, Mana::C]), &golem(2)), Ways::None));
    // Colorless can't pay X at all.
    assert!(matches!(ways(&pool(&[Mana::C, Mana::C, Mana::C]), &golem(1)), Ways::None));
    // White and blue can.
    assert!(matches!(ways(&pool(&[Mana::W, Mana::U, Mana::C, Mana::C]), &golem(2)), Ways::One([1, 1, 0, 0, 0, 2])));
    // Two whites, a blue and two colorless are five mana, but two colors: no X 3.
    assert!(matches!(ways(&pool(&[Mana::W, Mana::W, Mana::U, Mana::C, Mana::C]), &golem(3)), Ways::None));

    // Kicked from the pool: after a white has paid X, a second white is offered only for the {2}.
    let mut g = game();
    let card = put(&mut g, &GOLEM, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::W, Mana::W, Mana::B, Mana::R, Mana::C]);
    let r = announce(&mut g, card);
    assert_eq!(most_x(&g, &r), Some(3), "five mana, three colors: X 3, a white and the {{C}} paying the {{2}}");
    let r = announce_as(&mut g, &r, 2);
    let r = spend(&mut g, &r, Mana::W);
    let r = spend(&mut g, &r, Mana::W);
    let RequestKind::PayCosts { remaining, .. } = &r.kind else { unreachable!() };
    assert_eq!((remaining.generic, remaining.x_part()), (2, 1), "the second white paid the {{2}}, not X");
    let r = spend(&mut g, &r, Mana::B);
    spend(&mut g, &r, Mana::R);
    assert_eq!(g.state.stack.len(), 1, "the Golem is cast, kicked");
}

/// Four black mana in the pool, which pays any part of Drain's cost in one way.
fn four_black(g: &mut Game) {
    add_mana(g, &[Mana::B, Mana::B, Mana::B, Mana::B]);
}

#[test]
fn the_most_x_counts_a_cost_increase() {
    let mut g = game();
    put(&mut g, &TAXER, Zone::BATTLEFIELD);
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    four_black(&mut g);
    // {X}{1}{B} and {1} more (CR 601.2f): X 1 is four mana, X 2 five.
    let r = announce(&mut g, drain);
    assert_eq!(most_x(&g, &r), Some(1), "the tax leaves X 1, not the 2 the cost alone would");
    // And X 1 is paid.
    let r = announce_paid(&mut g, &r, 1);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "paid, got {:?}", r.kind);
    assert_eq!(g.state.stack.len(), 1, "Drain is cast");
}

#[test]
fn the_most_x_counts_a_cost_reduction() {
    let mut g = game();
    put(&mut g, &REDUCER, Zone::BATTLEFIELD);
    let drain = put(&mut g, &DRAIN, Zone::hand(YOU));
    four_black(&mut g);
    // {X}{1}{B} and {1} less (CR 601.2f): X 3 is four mana.
    let r = announce(&mut g, drain);
    assert_eq!(most_x(&g, &r), Some(3), "the reduction reaches X 3, not the 2 the cost alone would");
    let r = announce_paid(&mut g, &r, 3);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "paid, got {:?}", r.kind);
    assert_eq!(g.state.stack.len(), 1, "Drain is cast");
}

#[test]
fn the_most_x_counts_the_cards_exiled_from_the_hand_for_less() {
    let mut g = game();
    let march = put(&mut g, &MARCH, Zone::hand(YOU));
    let fodder = put(&mut g, &FODDER, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::B, Mana::B, Mana::B]);
    let r = next(&mut g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(march)).expect("it is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    // The card exiled for {2} less is chosen before X (CR 601.2b).
    let r = next(&mut g);
    g.answer(r.id, r.player, Answer::Cards(vec![fodder])).expect("a legal answer");
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, .. }), "expected the announcement of X, got {:?}", r.kind);
    // {X}{1}{B} less {2}, X's part first: X 3 is {1}{B} and X's {1}, three mana.
    assert_eq!(most_x(&g, &r), Some(3), "the {{2}} the card takes off reaches X 3");
    let r = announce_paid(&mut g, &r, 3);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "paid, got {:?}", r.kind);
    assert_eq!(g.state.stack.len(), 1, "it is cast");
}

#[test]
fn an_abilitys_x_is_judged_beside_a_kicker_that_is_not_x() {
    let mut g = game();
    let rats = put(&mut g, &KICKED_RATS, Zone::BATTLEFIELD);
    add_mana(&mut g, &[Mana::B, Mana::R]);
    let r = announce(&mut g, rats);
    assert_eq!(most_x(&g, &r), Some(2), "the card's kicker is its spell's, not the ability's X");
}

#[test]
fn the_most_x_counts_an_increase_the_targets_bring() {
    let mut g = game();
    put(&mut g, &WARDEN, Zone::BATTLEFIELD);
    put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let sap = put(&mut g, &SAP, Zone::hand(YOU));
    four_black(&mut g);
    // Its one target is a creature, so {2} more (CR 601.2c, 601.2f): X 1, not 3.
    let r = announce(&mut g, sap);
    assert_eq!(most_x(&g, &r), Some(1), "the tax its target brings leaves X 1");
}

static PROBE_FOREST: CardDef = CardDef {
    name: "X Mana Probe Forest",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..CardDef::BLANK
};

/// Birds of Paradise, near enough: a 0-power creature that taps for mana.
static PROBE_DORK: CardDef = CardDef {
    name: "X Mana Probe Dork",
    cost: mana("{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(1),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..CardDef::BLANK
};

/// A 2-power creature that taps for mana.
static PROBE_BIG_DORK: CardDef = CardDef {
    name: "X Mana Probe Big Dork",
    cost: mana("{1}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..CardDef::BLANK
};

/// Nature's Rhythm's harmonize: "Harmonize {X}{G}{G}{G}{G}", near enough.
static PROBE_RHYTHM: CardDef = CardDef {
    name: "X Mana Probe Rhythm",
    cost: mana("{X}{G}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], nothing),
    ways: &[way(Cost { mana: mana("{X}{G}"), tap_to_reduce: true, ..Cost::NONE }, ZoneKind::Graveyard, "Harmonize {X}{G}").exile_after()],
    ..CardDef::BLANK
};

/// Kessig Wolf Run, near enough: a land whose ability with {X} taps it, and which taps for mana itself.
static PROBE_RUN: CardDef = CardDef {
    name: "X Mana Probe Run",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}."), ability(cost("{X}{R}, {T}"), nothing, "{X}{R}, {T}: Nothing.")],
    ..CardDef::BLANK
};

/// "Waterbend {X}: Nothing." on a land: each untapped artifact or creature may pay {1} of X (CR 701.67a-b).
static PROBE_BENDER: CardDef = CardDef {
    name: "X Mana Probe Bender",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[ability(of_mana(mana("{X}")), nothing, "Waterbend {X}: Nothing.").waterbend()],
    ..CardDef::BLANK
};

fn probe_game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &PROBE_FOREST, &PROBE_DORK, &PROBE_BIG_DORK, &PROBE_RHYTHM, &PROBE_RUN, &PROBE_BENDER, &MOUNTAIN, &SWAMP, &BEAR,
    ])));
    let deck = || vec![table.id_of(&SWAMP); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

/// Casts `spell` from the graveyard by harmonize, tapping `creature` for it, and gives back the announcement of X.
fn harmonize(g: &mut Game, spell: ObjectId, creature: ObjectId) -> Request {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(spell)).expect("harmonize offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).unwrap();
    let r = next(g);
    assert!(matches!(r.kind, RequestKind::TapForPower { .. }), "expected the creature to tap, got {:?}", r.kind);
    g.answer(r.id, r.player, Answer::Cards(vec![creature])).unwrap();
    let r = next(g);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, .. }), "expected the announcement of X, got {:?}", r.kind);
    r
}

#[test]
fn the_creature_harmonize_taps_is_not_counted_as_mana() {
    let mut g = probe_game();
    put(&mut g, &PROBE_FOREST, Zone::BATTLEFIELD);
    let dork = put(&mut g, &PROBE_DORK, Zone::BATTLEFIELD);
    let spell = put(&mut g, &PROBE_RHYTHM, Zone::graveyard(YOU));
    let r = harmonize(&mut g, spell, dork);
    // The dork is tapped for harmonize (CR 702.180b, power 0); the Forest pays {G}: X can only be 0.
    assert_eq!(most_x(&g, &r), Some(0));
}

#[test]
fn the_power_of_the_creature_harmonize_taps_comes_off_x() {
    let mut g = probe_game();
    put(&mut g, &PROBE_FOREST, Zone::BATTLEFIELD);
    let dork = put(&mut g, &PROBE_BIG_DORK, Zone::BATTLEFIELD);
    let spell = put(&mut g, &PROBE_RHYTHM, Zone::graveyard(YOU));
    let r = harmonize(&mut g, spell, dork);
    // {X}{G} less {2} (CR 702.180a): X 2 is {G}, which the Forest pays; the tapped dork makes no mana.
    assert_eq!(most_x(&g, &r), Some(2));
}

#[test]
fn the_source_an_ability_taps_is_not_counted_as_mana() {
    let mut g = probe_game();
    put(&mut g, &MOUNTAIN, Zone::BATTLEFIELD);
    put(&mut g, &SWAMP, Zone::BATTLEFIELD);
    let run = put(&mut g, &PROBE_RUN, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(run) && o.index == 1).expect("offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).unwrap();
    let r = next(&mut g);
    // The Run taps for its own cost (CR 602.2b, 605.3b); the Mountain pays {R}, the Swamp X = 1.
    assert_eq!(most_x(&g, &r), Some(1));
}

#[test]
fn the_most_x_of_a_waterbend_cost_counts_what_may_be_tapped_for_it() {
    let mut g = probe_game();
    put(&mut g, &BEAR, Zone::BATTLEFIELD);
    put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let bender = put(&mut g, &PROBE_BENDER, Zone::BATTLEFIELD);
    let r = announce(&mut g, bender);
    // No mana, but each Bear may be tapped for {1} of X (CR 701.67a-b): X 2, as the priority menu judges it.
    assert_eq!(most_x(&g, &r), Some(2));
}
