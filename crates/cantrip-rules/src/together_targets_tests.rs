//! A target slot whose targets together are judged (`TargetSpec::together`, CR 601.2c, 115.1), over test cards and test
//! set rules: "any number of target creature cards that each have a different mana value X or less" (Agadeem's Awakening's
//! wording: "X or less" the slot's filter, reading the X announced, and "different mana values" its rule), "X target cards
//! with different mana values" (Long Rest's) and "up to three target creature cards with total mana value 3 or less" (Patch
//! Up's). A candidate the rule refuses alone is not offered, a set it refuses is refused and one
//! it allows taken, the host's check offers only what keeps the set legal, a count no set the rule allows reaches is no
//! legal announcement, and the set is judged again as the spell resolves (CR 608.2b): one the rule no longer allows makes
//! every target of the slot illegal (Nethroi, Apex of Death's ruling: "Return any number of target creature cards with total
//! power 10 or less", a power defined by a characteristic-defining ability growing), a target gone read as it last was (Run
//! Away Together's), where a target that loses a quality of its own is illegal alone. Whether a slot's fewest targets can be
//! chosen together is found exactly and quickly, however many cards there are.
use crate::{Game, Setup};
use cantrip_api::select::{Bits, Choice, LargestFn, SelectFn, Verdict, largest_distinct, largest_within, set_part, set_tallied};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind, Tally};

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Together Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

const fn creature(name: &'static str, cost: ManaCost) -> CardDef {
    CardDef { name, cost, has_cost: true, types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK }
}

static ONE: CardDef = creature("Together Test One", ManaCost { generic: 1, ..ManaCost::ZERO });

/// Ten creature cards of mana values 0 to 9, for a graveyard of many.
static RANGE: [CardDef; 10] = [
    creature("Together Test Range 0", ManaCost::ZERO),
    creature("Together Test Range 1", ManaCost { generic: 1, ..ManaCost::ZERO }),
    creature("Together Test Range 2", ManaCost { generic: 2, ..ManaCost::ZERO }),
    creature("Together Test Range 3", ManaCost { generic: 3, ..ManaCost::ZERO }),
    creature("Together Test Range 4", ManaCost { generic: 4, ..ManaCost::ZERO }),
    creature("Together Test Range 5", ManaCost { generic: 5, ..ManaCost::ZERO }),
    creature("Together Test Range 6", ManaCost { generic: 6, ..ManaCost::ZERO }),
    creature("Together Test Range 7", ManaCost { generic: 7, ..ManaCost::ZERO }),
    creature("Together Test Range 8", ManaCost { generic: 8, ..ManaCost::ZERO }),
    creature("Together Test Range 9", ManaCost { generic: 9, ..ManaCost::ZERO }),
];
static OTHER_ONE: CardDef = creature("Together Test Other One", ManaCost { generic: 1, ..ManaCost::ZERO });
static TWO: CardDef = creature("Together Test Two", ManaCost { generic: 2, ..ManaCost::ZERO });
static THREE: CardDef = creature("Together Test Three", ManaCost { generic: 3, ..ManaCost::ZERO });
static FIVE: CardDef = creature("Together Test Five", ManaCost { generic: 5, ..ManaCost::ZERO });

/// "Its power is equal to the number of cards in its owner's hand" (a characteristic-defining ability, CR 604.3), toughness 1.
fn hand_size(q: &dyn LayerQuery, id: ObjectId, chars: &mut Chars) {
    let owner = q.state().obj(id).owner;
    chars.power = Some(q.state().zone(Zone::hand(owner)).len() as i16);
    chars.toughness = Some(1);
}

static STAR: CardDef = CardDef {
    name: "Together Test Star",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Creature]),
    statics: &[characteristic_defining(Layer::PtDefine, hand_size, "Its power is equal to the number of cards in its owner's hand.")],
    ..CardDef::BLANK
};

fn different_mana_values(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let chosen: Vec<u32> = choice.selected.iter().map(|&c| q.mana_value(c)).collect();
    let complete = chosen.iter().enumerate().all(|(i, v)| !chosen[..i].contains(v));
    Verdict { complete, addable: Bits::of(choice.candidates.len(), |i| !chosen.contains(&q.mana_value(choice.candidates[i]))) }
}

fn total_mana_value(q: &dyn Query, choice: &Choice, most: u32) -> Verdict {
    let total: u64 = choice.selected.iter().map(|&c| u64::from(q.mana_value(c))).sum();
    let left = u64::from(most).saturating_sub(total);
    Verdict { complete: total <= u64::from(most), addable: Bits::of(choice.candidates.len(), |i| u64::from(q.mana_value(choice.candidates[i])) <= left) }
}

fn total_power(q: &dyn Query, choice: &Choice, most: u32) -> Verdict {
    let power = |c: ObjectId| i64::from(q.chars(c).power.unwrap_or(0));
    let total: i64 = choice.selected.iter().map(|&c| power(c)).sum();
    Verdict { complete: total <= i64::from(most), addable: Bits::of(choice.candidates.len(), |i| total + power(choice.candidates[i]) <= i64::from(most)) }
}

fn largest_different(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    largest_distinct(choice, |c| q.mana_value(c))
}

fn largest_total(q: &dyn Query, choice: &Choice, most: u32) -> Vec<ObjectId> {
    largest_within(choice, |c| u64::from(q.mana_value(c)), u64::from(most))
}

fn largest_power(q: &dyn Query, choice: &Choice, most: u32) -> Vec<ObjectId> {
    // Every test card's power is at least 0.
    largest_within(choice, |c| u64::try_from(q.chars(c).power.unwrap_or(0)).unwrap_or(0), u64::from(most))
}

const DIFFERENT: Part = set_part(&(different_mana_values as SelectFn), &(largest_different as LargestFn), 0, "with different mana values");
const TOTAL_POWER_AT_MOST_TWO: Part = set_part(&(total_power as SelectFn), &(largest_power as LargestFn), 2, "with total power {n} or less");
const TOTAL_AT_MOST_THREE: Part =
    set_tallied(&(total_mana_value as SelectFn), &(largest_total as LargestFn), 3, "with total mana value {n} or less", Tally::ManaValueAtMost);

fn creature_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone == Zone::graveyard(asker.controller) && q.chars(id).is(CardType::Creature))
}

/// A creature card in your graveyard with mana value X or less: a quality of each target, the filter's, reading the X
/// announced (none while it is only asked whether some X could have a target: any).
fn creature_card_x_or_less(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_card_in_your_graveyard(q, asker, t) && matches!(t, Target::Object(id) if q.announced_x().is_none_or(|x| q.mana_value(id) <= x))
}

/// A creature card in your graveyard with power X or less.
fn creature_card_power_x_or_less(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_card_in_your_graveyard(q, asker, t)
        && matches!(t, Target::Object(id) if q.announced_x().is_none_or(|x| i64::from(q.chars(id).power.unwrap_or(0)) <= i64::from(x)))
}

fn card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone == Zone::graveyard(asker.controller))
}

/// Puts each of its targets that is still legal onto the battlefield (CR 608.2b).
fn return_legal(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let targets = g.state().targets(item).to_vec();
    let cards: Vec<(ObjectId, PlayerId)> = targets
        .iter()
        .enumerate()
        .filter(|&(slot, _)| g.target_legal(item, slot))
        .filter_map(|(_, &t)| if let Target::Object(id) = t { Some((id, item.controller)) } else { None })
        .collect();
    g.put_all_onto_battlefield(&cards)?;
    Ok(())
}

static AWAKENING_TARGETS: [TargetSpec; 1] =
    [TargetSpec { min: 0, max: TargetSpec::ANY_NUMBER, ..target(creature_card_x_or_less) }.together(DIFFERENT)];

/// {X} sorcery: "Return from your graveyard to the battlefield any number of target creature cards that each have a different
/// mana value X or less."
static AWAKENING: CardDef = CardDef {
    name: "Together Test Awakening",
    cost: ManaCost { x: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&AWAKENING_TARGETS, return_legal),
    ..CardDef::BLANK
};

static REST_TARGETS: [TargetSpec; 1] = [target(card_in_your_graveyard).counted().together(DIFFERENT)];

/// {X} sorcery: "Return X target cards with different mana values from your graveyard to the battlefield."
static REST: CardDef = CardDef {
    name: "Together Test Rest",
    cost: ManaCost { x: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&REST_TARGETS, return_legal),
    ..CardDef::BLANK
};

static PATCH_TARGETS: [TargetSpec; 1] = [up_to(3, creature_card_in_your_graveyard).together(TOTAL_AT_MOST_THREE)];

static GATHER_TARGETS: [TargetSpec; 1] =
    [TargetSpec { min: 0, max: TargetSpec::ANY_NUMBER, ..target(creature_card_power_x_or_less) }.together(DIFFERENT)];

/// {X} sorcery: "Return any number of target creature cards with power X or less and different mana values from your
/// graveyard to the battlefield."
static GATHER: CardDef = CardDef {
    name: "Together Test Gather",
    cost: ManaCost { x: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&GATHER_TARGETS, return_legal),
    ..CardDef::BLANK
};

static SOZIN_TARGETS: [TargetSpec; 1] = [TargetSpec { min: 0, max: TargetSpec::ANY_NUMBER, ..target(creature_card_in_your_graveyard) }
    .together(set_tallied(&(total_mana_value as SelectFn), &(largest_total as LargestFn), TargetSpec::COUNTED, "with total mana value {n} or less", Tally::ManaValueAtMost))];

/// {X} instant: "Return any number of target creature cards with total mana value X or less from your graveyard to the
/// battlefield."
static SOZIN: CardDef = CardDef {
    name: "Together Test Sozin",
    cost: ManaCost { x: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&SOZIN_TARGETS, return_legal),
    ..CardDef::BLANK
};

/// "Choose new targets for target spell" (CR 115.7d), its controller choosing.
fn new_targets(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(Target::Object(spell)) = g.state().targets(item).first().copied() {
        g.choose_new_targets(spell, item.controller)?;
    }
    Ok(())
}

fn spell_on_stack(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Stack))
}

/// {0} instant: "You may choose new targets for target spell." (Redirect's kind, CR 115.7d.)
static REDIRECT: CardDef = CardDef {
    name: "Together Test Redirect",
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(spell_on_stack)], new_targets),
    ..CardDef::BLANK
};

/// {0} sorcery: "Return up to three target creature cards with total mana value 3 or less from your graveyard to the
/// battlefield."
static PATCH: CardDef = CardDef {
    name: "Together Test Patch",
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&PATCH_TARGETS, return_legal),
    ..CardDef::BLANK
};

static REUNION_TARGETS: [TargetSpec; 1] =
    [TargetSpec { min: 0, max: TargetSpec::ANY_NUMBER, ..target(creature_card_in_your_graveyard) }.together(TOTAL_POWER_AT_MOST_TWO)];

/// {0} sorcery: "Return any number of target creature cards with total power 2 or less from your graveyard to the
/// battlefield." (Nethroi, Apex of Death's and Reunion of the House's wording, at 2.)
static REUNION: CardDef = CardDef {
    name: "Together Test Reunion",
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&REUNION_TARGETS, return_legal),
    ..CardDef::BLANK
};

fn draw_two(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    Ok(())
}

/// {0} instant: "Draw two cards."
static DRAW: CardDef = CardDef { name: "Together Test Draw", has_cost: true, types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], draw_two), ..CardDef::BLANK };

/// Your precombat main phase with priority, nothing anywhere.
fn game() -> Game {
    let table: &'static CardTable =
        Box::leak(Box::new(CardTable::new(RANGE.iter().chain([&FILLER, &ONE, &OTHER_ONE, &TWO, &THREE, &FIVE, &STAR, &AWAKENING, &REST, &PATCH, &REUNION, &DRAW, &GATHER, &SOZIN, &REDIRECT]))));
    let deck = || vec![table.id_of(&FILLER); 20];
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
    // An empty hand, so that what is in it is only what a test puts there.
    for id in g.state.zone(Zone::hand(YOU)).to_vec() {
        g.state.zone_remove(Zone::hand(YOU), id);
        g.state.update_obj(id, |o| o.zone = Zone::exile(YOU));
        g.state.zone_push(Zone::exile(YOU), id);
    }
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id);
    id
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("a legal answer");
}

/// Your graveyard of One, Other One, Two and Three (mana values 1, 1, 2 and 3), and `spell` in your hand with `mana` in
/// your pool.
fn board(spell: &'static CardDef, mana: u32) -> (Game, ObjectId, [ObjectId; 4]) {
    let mut g = game();
    let graveyard = Zone::graveyard(YOU);
    let cards = [put(&mut g, &ONE, graveyard), put(&mut g, &OTHER_ONE, graveyard), put(&mut g, &TWO, graveyard), put(&mut g, &THREE, graveyard)];
    let card = put(&mut g, spell, Zone::hand(YOU));
    if mana > 0 {
        g.state.add_mana(YOU, Mana::C, mana, ObjectId(0), None, ManaLasts::Step);
    }
    (g, card, cards)
}

/// Casts `card`, announcing `x`, and gives back the question after the announcement.
fn cast(g: &mut Game, card: ObjectId, x: Option<u32>) -> Request {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(card)).expect("the spell is offered").clone();
    answer(g, &r, Answer::Act(Action::of(&option)));
    let mut r = next(g);
    if let Some(x) = x {
        assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, .. }), "expected the announcement of X, got {:?}", r.kind);
        answer(g, &r, Answer::Announce(x));
        r = next(g);
    }
    r
}

/// Pays the pool's one way, giving back the question after it.
fn pay(g: &mut Game, r: Request) -> Request {
    let RequestKind::PayCosts { options, .. } = &r.kind else { return r };
    let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
    answer(g, &r, Answer::Act(Action::of(confirm)));
    next(g)
}

/// Pays the pool's one way and passes until the stack is empty.
fn pay_and_resolve(g: &mut Game, r: Request) {
    let r = pay(g, r);
    pass_until_resolved(g, r);
}

/// Passes until the stack is empty.
fn pass_until_resolved(g: &mut Game, mut r: Request) {
    while !g.state.stack.is_empty() {
        assert!(matches!(r.kind, RequestKind::Priority { .. }), "expected priority, got {:?}", r.kind);
        answer(g, &r, Answer::Pass);
        r = next(g);
    }
}

fn objects(ids: &[ObjectId]) -> Vec<Target> {
    ids.iter().map(|&id| Target::Object(id)).collect()
}

/// Where each of `ids` is now: the object it became, if it changed zones.
fn zone_of(g: &Game, id: ObjectId) -> ZoneKind {
    let mut id = id;
    while let Some(next) = g.state.obj(id).became {
        id = next;
    }
    g.state.obj(id).zone.kind()
}

#[test]
fn a_candidate_the_rule_refuses_alone_is_not_offered() {
    let (mut g, awakening, [one, other_one, two, three]) = board(&AWAKENING, 2);
    let r = cast(&mut g, awakening, Some(2));
    let RequestKind::ChooseTarget { candidates, together, .. } = &r.kind else { panic!("expected the targets, got {:?}", r.kind) };
    assert_eq!(*candidates, objects(&[one, other_one, two]), "Three's mana value is over X 2: the filter, reading X, refuses it");
    assert!(!candidates.contains(&Target::Object(three)));
    assert_eq!(together.map(|p| p.words), Some("with different mana values"));
}

#[test]
fn a_set_the_rule_refuses_is_refused_and_one_it_allows_is_taken() {
    let (mut g, awakening, [one, other_one, two, _]) = board(&AWAKENING, 2);
    let r = cast(&mut g, awakening, Some(2));
    assert!(g.answer(r.id, r.player, Answer::Targets(objects(&[one, other_one]))).is_err(), "two of mana value 1 (CR 601.2c)");
    assert!(g.answer(r.id, r.player, Answer::Targets(objects(&[one, other_one, two]))).is_err());
    answer(&mut g, &r, Answer::Targets(objects(&[one, two])));
    let r = next(&mut g);
    pay_and_resolve(&mut g, r);
    assert_eq!(zone_of(&g, one), ZoneKind::Battlefield);
    assert_eq!(zone_of(&g, two), ZoneKind::Battlefield);
    assert_eq!(zone_of(&g, other_one), ZoneKind::Graveyard);
}

#[test]
fn the_check_as_the_player_picks_offers_only_what_keeps_the_set_legal() {
    let (mut g, awakening, [one, other_one, two, _]) = board(&AWAKENING, 2);
    let r = cast(&mut g, awakening, Some(2));
    let (cards, none) = crate::validate::selection_verdict(&g, &r, &[]).expect("a slot judged together is checked");
    assert_eq!(cards, vec![one, other_one, two]);
    assert!(none.complete && none.addable.ones().count() == 3, "any number: none is an answer, each may be picked first");
    let (_, after_one) = crate::validate::selection_verdict(&g, &r, &[one]).expect("a verdict");
    assert!(after_one.complete);
    assert_eq!(after_one.addable.ones().map(|i| cards[i]).collect::<Vec<_>>(), vec![two], "Other One shares One's mana value");
    let (_, both_ones) = crate::validate::selection_verdict(&g, &r, &[one, other_one]).expect("a verdict");
    assert!(!both_ones.complete);
    // A slot that asks nothing of its targets together is no choice of cards to check.
    let (mut plain, card, _) = board(&PATCH, 0);
    let r = cast(&mut plain, card, None);
    let RequestKind::ChooseTarget { together, .. } = &r.kind else { panic!("expected the targets, got {:?}", r.kind) };
    assert!(together.is_some());
}

#[test]
fn a_set_the_rule_no_longer_allows_as_the_spell_resolves_makes_every_target_illegal() {
    let mut g = game();
    let graveyard = Zone::graveyard(YOU);
    let star = put(&mut g, &STAR, graveyard);
    let one = put(&mut g, &ONE, graveyard);
    let reunion = put(&mut g, &REUNION, Zone::hand(YOU));
    let draw = put(&mut g, &DRAW, Zone::hand(YOU));
    let r = cast(&mut g, reunion, None);
    // The Star's power is 1, the Draw in hand: with One's 1, total power 2.
    answer(&mut g, &r, Answer::Targets(objects(&[star, one])));
    let r = next(&mut g);
    let r = pay(&mut g, r);
    // In response, two cards drawn: the Star's power becomes 2, and the targets' total 3, over 2 (CR 608.2b).
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(draw)).expect("the Draw is offered").clone();
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    pay_and_resolve(&mut g, r);
    assert_eq!(zone_of(&g, one), ZoneKind::Graveyard, "the whole selection is illegal (Nethroi's ruling): One is not returned");
    assert_eq!(zone_of(&g, star), ZoneKind::Graveyard);
    assert_eq!(zone_of(&g, reunion), ZoneKind::Graveyard, "with every target illegal it doesn't resolve");
}

#[test]
fn a_set_the_rule_still_allows_resolves() {
    let mut g = game();
    let graveyard = Zone::graveyard(YOU);
    let star = put(&mut g, &STAR, graveyard);
    let one = put(&mut g, &ONE, graveyard);
    let reunion = put(&mut g, &REUNION, Zone::hand(YOU));
    let r = cast(&mut g, reunion, None);
    let RequestKind::ChooseTarget { candidates, .. } = &r.kind else { panic!("expected the targets, got {:?}", r.kind) };
    assert_eq!(candidates.len(), 2);
    answer(&mut g, &r, Answer::Targets(objects(&[star, one])));
    let r = next(&mut g);
    pay_and_resolve(&mut g, r);
    assert_eq!((zone_of(&g, star), zone_of(&g, one)), (ZoneKind::Battlefield, ZoneKind::Battlefield));
}

#[test]
fn a_target_gone_is_still_judged_with_the_others_as_it_last_was() {
    let (mut g, patch, [one, _, two, _]) = board(&PATCH, 0);
    let r = cast(&mut g, patch, None);
    answer(&mut g, &r, Answer::Targets(objects(&[one, two])));
    let r = next(&mut g);
    pay(&mut g, r);
    let item = g.state.stack[0];
    let legal = |g: &Game, slot: usize| crate::resolve::target_legal(g.cards, &g.state, &g.log, &item, slot);
    assert!(legal(&g, 0) && legal(&g, 1), "total mana value 3");
    // One leaves the graveyard: it is illegal, and Two, with One as it last was, still within 3 (Run Away Together's ruling).
    {
        let mut cx = crate::Cx { g: &mut g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
        crate::ops::move_to_for(&mut cx, one, Zone::exile(YOU), ZoneReason::Other);
    }
    assert!(!legal(&g, 0) && legal(&g, 1));
    // Two's mana value becomes 3: with One's 1 as it last was, the total is 4, and Two is illegal too.
    let three = g.cards.id_of(&THREE);
    g.state.update_obj(two, |o| o.card = three);
    assert!(!legal(&g, 0) && !legal(&g, 1), "One, gone, still counts as it last was");
}

#[test]
fn a_total_mana_value_still_limits_the_targets_together() {
    let (mut g, patch, [one, other_one, two, three]) = board(&PATCH, 0);
    let r = cast(&mut g, patch, None);
    let RequestKind::ChooseTarget { candidates, count, .. } = &r.kind else { panic!("expected the targets, got {:?}", r.kind) };
    assert_eq!((candidates.len(), *count), (4, 3), "Three is a target alone");
    assert!(g.answer(r.id, r.player, Answer::Targets(objects(&[two, one, other_one]))).is_err(), "total 4, 3 or less wanted");
    assert!(g.answer(r.id, r.player, Answer::Targets(objects(&[three, one]))).is_err());
    answer(&mut g, &r, Answer::Targets(objects(&[two, one])));
    let r = next(&mut g);
    pay_and_resolve(&mut g, r);
    assert_eq!((zone_of(&g, two), zone_of(&g, one), zone_of(&g, three)), (ZoneKind::Battlefield, ZoneKind::Battlefield, ZoneKind::Graveyard));
}

#[test]
fn a_count_no_set_the_rule_allows_reaches_is_no_legal_announcement() {
    // X 3 of cards with different mana values, among mana values 1, 1, 2 and 3: One, Two and Three.
    let (mut g, rest, [one, other_one, two, three]) = board(&REST, 3);
    let r = cast(&mut g, rest, Some(3));
    let RequestKind::ChooseTarget { least, .. } = &r.kind else { panic!("expected the targets, got {:?}", r.kind) };
    assert_eq!(*least, 3);
    assert!(g.answer(r.id, r.player, Answer::Targets(objects(&[one, other_one, two]))).is_err());
    answer(&mut g, &r, Answer::Targets(objects(&[other_one, two, three])));
    // Without Three there are three cards and only two mana values: X 3 has no legal choice (CR 601.2c), so the cast is
    // taken back.
    let (mut g, rest, [_, _, _, three]) = board(&REST, 3);
    g.state.zone_remove(Zone::graveyard(YOU), three);
    g.state.update_obj(three, |o| o.zone = Zone::exile(YOU));
    let r = cast(&mut g, rest, Some(3));
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "expected the cast taken back, got {:?}", r.kind);
    assert_eq!(g.state.obj(rest).zone, Zone::hand(YOU));
}

#[test]
fn a_target_that_loses_a_quality_of_its_own_is_illegal_alone() {
    let mut g = game();
    let graveyard = Zone::graveyard(YOU);
    let star = put(&mut g, &STAR, graveyard);
    let two = put(&mut g, &TWO, graveyard);
    let gather = put(&mut g, &GATHER, Zone::hand(YOU));
    let draw = put(&mut g, &DRAW, Zone::hand(YOU));
    g.state.add_mana(YOU, Mana::C, 1, ObjectId(0), None, ManaLasts::Step);
    let r = cast(&mut g, gather, Some(1));
    // The Star's power is 1 (the Draw in hand), Two's 1: both have power X 1 or less, and different mana values.
    answer(&mut g, &r, Answer::Targets(objects(&[star, two])));
    let r = next(&mut g);
    let r = pay(&mut g, r);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(draw)).expect("the Draw is offered").clone();
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    // Two cards drawn: the Star's power is 2, over X. It alone is illegal (CR 608.2b); Two, still within the filter and with
    // a mana value of its own, is returned.
    pay_and_resolve(&mut g, r);
    assert_eq!(zone_of(&g, star), ZoneKind::Graveyard);
    assert_eq!(zone_of(&g, two), ZoneKind::Battlefield);
}

#[test]
fn whether_a_count_can_be_reached_together_is_found_at_once_among_many_cards() {
    // Fifty cards, five of each of ten mana values, and X 11 of them with different mana values: no such set, which a search
    // through the sets of different mana values (6^10 of them) would take minutes to rule out. X 10 has one.
    for (x, reachable) in [(11, false), (10, true)] {
        let mut g = game();
        for _ in 0..5 {
            for def in &RANGE {
                put(&mut g, def, Zone::graveyard(YOU));
            }
        }
        let rest = put(&mut g, &REST, Zone::hand(YOU));
        g.state.add_mana(YOU, Mana::C, x, ObjectId(0), None, ManaLasts::Step);
        let r = cast(&mut g, rest, Some(x));
        if reachable {
            let RequestKind::ChooseTarget { candidates, least, .. } = &r.kind else { panic!("expected the targets, got {:?}", r.kind) };
            assert_eq!((candidates.len(), *least), (50, 10));
            let (_, none) = crate::validate::selection_verdict(&g, &r, &[]).expect("a slot judged together");
            assert!(!none.complete && none.addable.ones().count() == 50);
        } else {
            assert!(matches!(r.kind, RequestKind::Priority { .. }), "X 11 has no legal choice, got {:?}", r.kind);
            assert_eq!(g.state.obj(rest).zone, Zone::hand(YOU));
        }
    }
}

#[test]
fn new_targets_for_a_spell_are_judged_together_with_the_count_it_was_cast_with() {
    let (mut g, sozin, [one, other_one, two, three]) = board(&SOZIN, 3);
    let redirect = put(&mut g, &REDIRECT, Zone::hand(YOU));
    let r = cast(&mut g, sozin, Some(3));
    answer(&mut g, &r, Answer::Targets(objects(&[one, two])));
    let r = next(&mut g);
    let r = pay(&mut g, r);
    let on_stack = g.state.stack[0].object;
    assert_eq!(g.state.stack[0].counted, 3, "the count its targets were chosen with (StackItem::counted)");
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(redirect)).expect("the Redirect is offered").clone();
    let mut action = Action::of(&option);
    action.targets = vec![Target::Object(on_stack)];
    answer(&mut g, &r, Answer::Act(action));
    let mut r = next(&mut g);
    r = pay(&mut g, r);
    // Passed until the Redirect resolves and asks for the Sozin's new targets (CR 115.7d), its total X 3 as it was cast.
    while !matches!(r.kind, RequestKind::ChooseTarget { .. }) {
        assert!(matches!(r.kind, RequestKind::Priority { .. }), "expected priority, got {:?}", r.kind);
        answer(&mut g, &r, Answer::Pass);
        r = next(&mut g);
    }
    let RequestKind::ChooseTarget { together, .. } = &r.kind else { unreachable!() };
    assert_eq!(together.map(|p| p.n), Some(3));
    assert!(g.answer(r.id, r.player, Answer::Targets(objects(&[three, one]))).is_err(), "total 4, over X 3");
    answer(&mut g, &r, Answer::Targets(objects(&[other_one, two])));
    let r = next(&mut g);
    pass_until_resolved(&mut g, r);
    assert_eq!((zone_of(&g, other_one), zone_of(&g, two)), (ZoneKind::Battlefield, ZoneKind::Battlefield));
    assert_eq!(zone_of(&g, one), ZoneKind::Graveyard);
}
