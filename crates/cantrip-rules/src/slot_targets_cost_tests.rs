//! A spell or ability whose cost reads its targets, or what is announced before them, over test cards: the total cost is
//! determined once the targets are announced (CR 601.2c, 601.2f, 602.2b), after its modes, X, kicker, gift and splices
//! (CR 601.2b), so it may be begun if some legal announcement of all of them could be paid for, and one that leaves the
//! cost unpayable takes it back (CR 601.2h, 733.1).
use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Slot Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static BEAR: CardDef = CardDef {
    name: "Slot Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield))
}

/// "This spell costs {3} less to cast if it targets a permanent you control" (This Town Ain't Big Enough's), and the same for
/// an ability of the card.
fn less_if_yours(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    let yours = targets.iter().any(|&t| matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield) && q.obj(id).controller == player));
    if yours { CostDelta::less(3) } else { CostDelta::NONE }
}

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield) && q.chars(id).is(CardType::Creature))
}

fn gain_three(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}

const FOUR: ManaCost = ManaCost { generic: 4, ..ManaCost::ZERO };

/// {4} instant: "This spell costs {3} less to cast if it targets a permanent you control. Up to two target permanents: you
/// gain 3 life."
static TOWN: CardDef = CardDef {
    name: "Slot Test Town",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[up_to(2, permanent)], gain_three),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

/// The same, with two slots of one target each: "target permanent and target permanent".
static PAIR: CardDef = CardDef {
    name: "Slot Test Pair",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(permanent), target(permanent)], gain_three),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

/// The same, with its one target announced with the cast: "target permanent".
static SINGLE: CardDef = CardDef {
    name: "Slot Test Single",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(permanent)], gain_three),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

/// "{4}: Up to two target creatures: you gain 3 life. This ability costs {3} less to activate if it targets a permanent you
/// control."
static ENGINE: CardDef = CardDef {
    name: "Slot Test Engine",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{4}"), gain_three, "{4}: Up to two target creatures: you gain 3 life.").targets(&[up_to(2, creature)])],
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

fn player(_: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Player(_))
}

/// An opponent's permanent, or any permanent once the spell was kicked (as Bloodchief's Thirst reads its kicker).
fn theirs_unless_kicked(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield) && (q.obj(asker.source).kicked > 0 || q.obj(id).controller != asker.controller))
}

/// "This spell costs {4} less to cast if it has two or more targets."
fn less_for_two(_: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    if targets.iter().filter(|&&t| t != Target::None).count() >= 2 { CostDelta::less(4) } else { CostDelta::NONE }
}

/// "Choose one - Target player: you gain 3 life. / {2} - Target permanent: you gain 3 life." (the second a spree-like mode
/// that costs {2} more).
static CHOICE_MODES: ModalDef = modes(&[
    mode(gain_three, "Target player: you gain 3 life.").targets(&[target(player)]),
    mode(gain_three, "Target permanent: you gain 3 life.").targets(&[target(permanent)]).cost(ManaCost { generic: 2, ..ManaCost::ZERO }),
]);

/// {4} sorcery, modal: "This spell costs {3} less to cast if it targets a permanent you control."
static CHOICE: CardDef = CardDef {
    name: "Slot Test Choice",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: Some(SpellDef::MODAL),
    modal: Some(&CHOICE_MODES),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

static X_PERMANENTS: [TargetSpec; 1] = [target(permanent).counted()];

/// {X}{3} sorcery: "This spell costs {3} less to cast if it targets a permanent you control. X target permanents: you gain 3
/// life."
static SWARM: CardDef = CardDef {
    name: "Slot Test Swarm",
    cost: ManaCost { generic: 3, x: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&X_PERMANENTS, gain_three),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

/// {4} sorcery, kicker {1}: "This spell costs {3} less to cast if it targets a permanent you control. Target permanent an
/// opponent controls, or if this spell was kicked, any target permanent: you gain 3 life."
static STRIKE: CardDef = CardDef {
    name: "Slot Test Strike",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[target(theirs_unless_kicked)], gain_three),
    late_targets: true,
    kicker: Some(Kicker { cost: ManaCost { generic: 1, ..ManaCost::ZERO }, extra: None, multi: false, waterbend: false }),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

static ONE_OR_KICKED_TWO: [TargetSpec; 1] = [TargetSpec { if_kicked: Some((2, 2)), ..target(permanent) }];

/// {4} sorcery, kicker {1}: "This spell costs {4} less to cast if it has two or more targets. Target permanent, or if this
/// spell was kicked, two target permanents: you gain 3 life."
static DOUBLE: CardDef = CardDef {
    name: "Slot Test Double",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&ONE_OR_KICKED_TWO, gain_three),
    kicker: Some(Kicker { cost: ManaCost { generic: 1, ..ManaCost::ZERO }, extra: None, multi: false, waterbend: false }),
    own_cost_changes: &[less_for_two],
    ..CardDef::BLANK
};

/// An opponent's permanent, or any permanent where the gift is promised (as `gift::promised` reads it).
fn theirs_unless_promised(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield)
        && (q.obj(asker.source).gift.is_some() || q.gift_assumed() || q.obj(id).controller != asker.controller))
}

fn give_nothing(_: &mut dyn cantrip_api::Game, _: PlayerId) -> Result<(), Suspend> {
    Ok(())
}

/// {4} sorcery, gift a card: "This spell costs {3} less to cast if it targets a permanent you control. Target permanent an
/// opponent controls, or if the gift was promised, any target permanent: you gain 3 life."
static PROMISE: CardDef = CardDef {
    name: "Slot Test Promise",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[target(theirs_unless_promised)], gain_three),
    gift: Some(GiftDef { kind: Gift::Card, give: give_nothing }),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

static ONE_OR_TWO: [TargetSpec; 1] = [target(permanent).between(1, 2)];

/// {4} sorcery: "This spell costs {3} less to cast if it targets a permanent you control. You gain 3 life, 2 divided as you
/// choose among one or two target permanents."
static SPLIT: CardDef = CardDef {
    name: "Slot Test Split",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&ONE_OR_TWO, gain_three),
    divide: Some(Divide { total: 2, of_x: None }),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

/// {4} Arcane instant: "This spell costs {3} less to cast if it targets a permanent you control. You gain 3 life."
static RITE: CardDef = CardDef {
    name: "Slot Test Rite",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    subtypes: SubtypeSet::of(&[Subtype::Arcane]),
    spell: spell(&[], gain_three),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

/// {4} instant: "Target permanent: you gain 3 life. Splice onto Arcane {1}."
static GRAFT: CardDef = CardDef {
    name: "Slot Test Graft",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(permanent)], gain_three),
    splice: Some(ManaCost { generic: 1, ..ManaCost::ZERO }),
    ..CardDef::BLANK
};

/// "This spell costs {3} less to cast if it was kicked."
fn less_if_kicked(q: &dyn Query, spell: ObjectId, _: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if matches!(costed, Costed::Spell(_)) && q.obj(spell).kicked > 0 { CostDelta::less(3) } else { CostDelta::NONE }
}

/// {4} instant, kicker {1}: "This spell costs {3} less to cast if it was kicked. Target permanent: you gain 3 life."
static SURGE: CardDef = CardDef {
    name: "Slot Test Surge",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(permanent)], gain_three),
    kicker: Some(Kicker { cost: ManaCost { generic: 1, ..ManaCost::ZERO }, extra: None, multi: false, waterbend: false }),
    own_cost_changes: &[less_if_kicked],
    ..CardDef::BLANK
};

/// Wings bestowed: an Aura with "enchant creature" (CR 702.103b), and "This spell costs {3} less to cast if it targets a
/// permanent you control."
static WINGS_AURA: CardDef = CardDef {
    name: "Slot Test Wings",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(target(creature)),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

/// A {4} creature with bestow {4}.
static WINGS: CardDef = CardDef {
    name: "Slot Test Wings",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    bestow: Some(&WINGS_AURA),
    bestow_cost: FOUR,
    ..CardDef::BLANK
};

/// "Choose one - Target player: you gain 3 life. / Target creature: you gain 3 life."
static DIAL_MODES: ModalDef = modes(&[
    mode(gain_three, "Target player: you gain 3 life.").targets(&[target(player)]),
    mode(gain_three, "Target creature: you gain 3 life.").targets(&[target(creature)]),
]);

/// "{4}: Choose one - ... This ability costs {3} less to activate if it targets a permanent you control."
static DIAL: CardDef = CardDef {
    name: "Slot Test Dial",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[modal_ability(cost("{4}"), &DIAL_MODES, "{4}: Choose one.")],
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

static X_CREATURES: [TargetSpec; 1] = [target(creature).counted()];

/// "{X}{3}: X target creatures: you gain 3 life. This ability costs {3} less to activate if it targets a permanent you
/// control."
static HIVE: CardDef = CardDef {
    name: "Slot Test Hive",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{X}{3}"), gain_three, "{X}{3}: X target creatures: you gain 3 life.").targets(&X_CREATURES)],
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

/// "This spell costs {1} less to cast for each target you control."
fn less_for_each_yours(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    let yours = targets.iter().filter(|&&t| matches!(t, Target::Object(id) if q.obj(id).controller == player)).count();
    CostDelta::less(yours.min(255) as u8)
}

static ANY_CREATURES: [TargetSpec; 1] = [target(creature).between(0, TargetSpec::ANY_NUMBER)];

/// {9} sorcery: "This spell costs {1} less to cast for each target you control. Any number of target creatures: you gain 3
/// life."
static BULK: CardDef = CardDef {
    name: "Slot Test Bulk",
    cost: ManaCost { generic: 9, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&ANY_CREATURES, gain_three),
    own_cost_changes: &[less_for_each_yours],
    ..CardDef::BLANK
};

/// Fireball's "This spell costs {1} more to cast for each target beyond the first."
fn more_per_extra_target(_: &dyn Query, _: ObjectId, _: PlayerId, costed: Costed, targets: &[Target]) -> CostDelta {
    let chosen = targets.iter().filter(|t| **t != Target::None).count();
    match costed {
        Costed::Spell(_) => CostDelta::more(chosen.saturating_sub(1).min(255) as u8),
        _ => CostDelta::NONE,
    }
}

/// {1} sorcery: "This spell costs {1} more to cast for each target beyond the first. Any number of target creatures: you
/// gain 3 life."
static VOLLEY: CardDef = CardDef {
    name: "Slot Test Volley",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&ANY_CREATURES, gain_three),
    own_cost_changes: &[more_per_extra_target],
    ..CardDef::BLANK
};

/// Two modes where the board allows it (as Flame of Anor's "if you control a Wizard"): here, while its caster controls a
/// creature.
fn two_with_a_creature(q: &dyn Query, asker: &Asker) -> u8 {
    if q.state().battlefield().iter().any(|&id| q.obj(id).controller == asker.controller && q.chars(id).is(CardType::Creature)) { 2 } else { 1 }
}

/// "Choose one. (Two while you control a creature.) - Target player: you gain 3 life. / Target creature: you gain 3 life."
static PAIRS_MODES: ModalDef = modes(&[
    mode(gain_three, "Target player: you gain 3 life.").targets(&[target(player)]),
    mode(gain_three, "Target creature: you gain 3 life.").targets(&[target(creature)]),
]);

/// {4} sorcery, modal, "This spell costs {4} less to cast if it has two or more targets."
static PAIRS: CardDef = CardDef {
    name: "Slot Test Pairs",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: Some(SpellDef::MODAL),
    modal: Some(&PAIRS_MODES),
    modes_up_to: Some(two_with_a_creature),
    own_cost_changes: &[less_for_two],
    ..CardDef::BLANK
};

static X_DIVIDED: [TargetSpec; 1] = [target(creature).between(1, TargetSpec::ANY_NUMBER)];

/// {4} instant, Fire Covenant's shape: "As an additional cost to cast this spell, pay X life. This spell costs {3} less to
/// cast if it targets a permanent you control. You gain 3 life, X divided as you choose among any number of target
/// creatures."
static COVENANT: CardDef = CardDef {
    name: "Slot Test Covenant",
    cost: FOUR,
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&X_DIVIDED, gain_three),
    additional_cost: Some(Cost { pay_x_life: true, ..Cost::NONE }),
    divide: Some(Divide { total: TargetSpec::COUNTED, of_x: None }),
    own_cost_changes: &[less_if_yours],
    ..CardDef::BLANK
};

/// "{9}: Any number of target creatures: you gain 3 life. This ability costs {1} less to activate for each target you
/// control."
static GRID: CardDef = CardDef {
    name: "Slot Test Grid",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{9}"), gain_three, "{9}: Any number of target creatures: you gain 3 life.").targets(&ANY_CREATURES)],
    own_cost_changes: &[less_for_each_yours],
    ..CardDef::BLANK
};

/// Your precombat main phase with priority, nothing anywhere.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &BEAR, &TOWN, &PAIR, &SINGLE, &ENGINE, &CHOICE, &SWARM, &STRIKE, &DOUBLE, &PROMISE, &SPLIT, &RITE, &GRAFT, &SURGE, &WINGS, &WINGS_AURA, &DIAL, &HIVE, &BULK, &VOLLEY, &PAIRS, &COVENANT, &GRID])));
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
    g
}

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, zone));
    g.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn add_mana(g: &mut Game, n: u32) {
    g.state.add_mana(YOU, Mana::C, n, ObjectId(0), None, ManaLasts::Step);
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("a legal answer");
}

fn offered(g: &mut Game, source: ObjectId) -> Option<ActionOption> {
    priority_options(&mut cx(g), YOU).into_iter().find(|o| o.source == Some(source))
}

/// Your Bear and theirs, and `def` in your hand (or on your battlefield) with one mana in your pool.
fn board(def: &'static CardDef, mine: bool) -> (Game, ObjectId, ObjectId, ObjectId) {
    let mut g = game();
    let theirs = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let yours = if mine { put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD) } else { theirs };
    let zone = if def.types.has(CardType::Artifact) { Zone::BATTLEFIELD } else { Zone::hand(YOU) };
    let card = put(&mut g, def, YOU, zone);
    add_mana(&mut g, 1);
    (g, card, yours, theirs)
}

/// Begins the action, answers its targets with `targets` slot by slot, and gives back the question after them.
fn begin(g: &mut Game, option: &ActionOption, slots: &[Vec<Target>]) -> Request {
    let r = next(g);
    answer(g, &r, Answer::Act(Action::of(option)));
    let mut r = next(g);
    for chosen in slots {
        assert!(matches!(r.kind, RequestKind::ChooseTarget { .. }), "expected a target slot, got {:?}", r.kind);
        answer(g, &r, Answer::Targets(chosen.clone()));
        r = next(g);
    }
    r
}

/// Pays the pool's one way and passes until the stack is empty.
fn pay_and_resolve(g: &mut Game, mut r: Request) {
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
        answer(g, &r, Answer::Act(Action::of(confirm)));
        r = next(g);
    }
    while !g.state.stack.is_empty() {
        assert!(matches!(r.kind, RequestKind::Priority { .. }), "expected priority, got {:?}", r.kind);
        answer(g, &r, Answer::Pass);
        r = next(g);
    }
}

#[test]
fn a_spell_of_up_to_two_targets_is_offered_for_what_a_target_of_its_casters_makes_it_cost() {
    let (mut g, town, yours, theirs) = board(&TOWN, true);
    let option = offered(&mut g, town).expect("{1} casts it at a permanent its caster controls (CR 601.2c, 601.2f)");
    let life = g.state.player(YOU).life;
    let r = begin(&mut g, &option, &[vec![yours, theirs].into_iter().map(Target::Object).collect()]);
    pay_and_resolve(&mut g, r);
    assert_eq!(g.state.player(YOU).life, life + 3);
    assert_eq!(g.state.player(YOU).pool.total(), 0, "{{1}} was paid");
}

#[test]
fn a_spell_of_up_to_two_targets_is_not_offered_where_no_target_makes_it_affordable() {
    let (mut g, town, _, _) = board(&TOWN, false);
    assert!(offered(&mut g, town).is_none(), "only the opponent's Bear: it costs {{4}} at any choice of targets");
    add_mana(&mut g, 3);
    assert!(offered(&mut g, town).is_some(), "{{4}} pays for it at full cost");
}

#[test]
fn targets_that_leave_the_cost_unpayable_take_the_cast_back() {
    let (mut g, town, _, theirs) = board(&TOWN, true);
    let option = offered(&mut g, town).expect("offered");
    let life = g.state.player(YOU).life;
    let r = begin(&mut g, &option, &[vec![Target::Object(theirs)]]);
    // {4} with one mana: nothing pays it, and the cast is given up (CR 601.2h, 733.1).
    let RequestKind::PayCosts { options, .. } = &r.kind else { panic!("expected the payment, got {:?}", r.kind) };
    assert!(!options.iter().any(|o| o.kind == ActionKind::ConfirmCasting));
    let cancel = options.iter().find(|o| o.kind == ActionKind::CancelCasting).expect("the cast may be taken back");
    answer(&mut g, &r, Answer::Act(Action::of(cancel)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "back to priority, got {:?}", r.kind);
    assert!(g.state.stack.is_empty());
    assert!(g.state.obj(town).zone.is(ZoneKind::Hand), "the spell returns to the hand");
    assert_eq!(g.state.player(YOU).pool.total(), 1, "no payment was made");
    assert_eq!(g.state.player(YOU).life, life);
    // And it may be cast again, at the Bear its caster controls.
    assert!(offered(&mut g, town).is_some());
}

#[test]
fn a_spell_of_two_slots_is_offered_for_what_a_target_of_its_casters_makes_it_cost() {
    let (mut g, pair, yours, theirs) = board(&PAIR, true);
    let option = offered(&mut g, pair).expect("{1} casts it with a slot at a permanent its caster controls");
    let life = g.state.player(YOU).life;
    let r = begin(&mut g, &option, &[vec![Target::Object(theirs)], vec![Target::Object(yours)]]);
    pay_and_resolve(&mut g, r);
    assert_eq!(g.state.player(YOU).life, life + 3);
    let (mut g, pair, _, _) = board(&PAIR, false);
    assert!(offered(&mut g, pair).is_none(), "both slots at the opponent's Bear cost {{4}}");
}

#[test]
fn a_spell_of_one_target_announced_with_the_cast_keeps_the_targets_it_could_be_paid_for() {
    let (mut g, single, yours, _) = board(&SINGLE, true);
    let option = offered(&mut g, single).expect("offered at the caster's Bear");
    assert_eq!(option.target_candidates, vec![Target::Object(yours)]);
    let (mut g, single, _, _) = board(&SINGLE, false);
    assert!(offered(&mut g, single).is_none());
}

#[test]
fn an_ability_of_up_to_two_targets_is_offered_for_what_a_target_of_its_controllers_makes_it_cost() {
    let (mut g, engine, yours, _) = board(&ENGINE, true);
    let option = offered(&mut g, engine).expect("{1} activates it at a permanent its controller controls (CR 602.2b)");
    let life = g.state.player(YOU).life;
    let r = begin(&mut g, &option, &[vec![Target::Object(yours)]]);
    pay_and_resolve(&mut g, r);
    assert_eq!(g.state.player(YOU).life, life + 3);
    let (mut g, engine, _, _) = board(&ENGINE, false);
    assert!(offered(&mut g, engine).is_none(), "only the opponent's Bear: it costs {{4}} at any choice of targets");
}

#[test]
fn a_modal_spell_is_offered_for_the_mode_and_target_that_make_it_affordable_with_the_modes_cost() {
    let (mut g, choice, yours, _) = board(&CHOICE, true);
    assert!(offered(&mut g, choice).is_none(), "the permanent mode costs {{2}} more: {{3}} at the caster's Bear, {{4}} at a player");
    add_mana(&mut g, 2);
    let option = offered(&mut g, choice).expect("{3} casts the permanent mode at the caster's Bear (CR 601.2b-c, 601.2f)");
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseModes { .. }), "the modes are asked, got {:?}", r.kind);
    answer(&mut g, &r, Answer::Modes(vec![1]));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Targets(vec![Target::Object(yours)]));
    let life = g.state.player(YOU).life;
    let r = next(&mut g);
    pay_and_resolve(&mut g, r);
    assert_eq!(g.state.player(YOU).life, life + 3);
    assert_eq!(g.state.player(YOU).pool.total(), 0, "{{3}} was paid");
    let (mut g, choice, _, _) = board(&CHOICE, false);
    add_mana(&mut g, 2);
    assert!(offered(&mut g, choice).is_none(), "no permanent of the caster's: {{4}} or {{6}}");
}

#[test]
fn a_spell_of_x_targets_is_offered_for_the_x_whose_targets_make_it_affordable() {
    let (mut g, swarm, yours, _) = board(&SWARM, true);
    let option = offered(&mut g, swarm).expect("X 1 at the caster's Bear costs {1} (CR 601.2b-c, 601.2f)");
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, .. }), "expected the announcement of X, got {:?}", r.kind);
    answer(&mut g, &r, Answer::Announce(1));
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Targets(vec![Target::Object(yours)]));
    let life = g.state.player(YOU).life;
    let r = next(&mut g);
    pay_and_resolve(&mut g, r);
    assert_eq!(g.state.player(YOU).life, life + 3);
    let (mut g, swarm, _, _) = board(&SWARM, false);
    assert!(offered(&mut g, swarm).is_none(), "X 0 costs {{3}}, and any X with targets the opponent's {{3}} and more");
}

#[test]
fn a_spell_whose_kicker_widens_its_target_is_offered_kicked_at_what_makes_it_affordable() {
    let (mut g, strike, _, _) = board(&STRIKE, true);
    assert!(offered(&mut g, strike).is_none(), "kicked at the caster's Bear it costs {{2}}");
    add_mana(&mut g, 1);
    assert!(offered(&mut g, strike).is_some(), "{{2}} casts it kicked at the caster's Bear (CR 601.2b-c, 601.2f)");
    let (mut g, strike, _, _) = board(&STRIKE, false);
    add_mana(&mut g, 1);
    assert!(offered(&mut g, strike).is_none(), "only the opponent's Bear: {{4}}, or {{5}} kicked");
}

#[test]
fn a_spell_whose_kicker_adds_a_target_is_offered_kicked_where_two_targets_make_it_affordable() {
    let (mut g, double, _, _) = board(&DOUBLE, true);
    assert!(offered(&mut g, double).is_some(), "kicked with two targets it costs {{1}} (CR 601.2b-c, 601.2f)");
    let mut g = game();
    put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let double = put(&mut g, &DOUBLE, YOU, Zone::hand(YOU));
    add_mana(&mut g, 1);
    assert!(offered(&mut g, double).is_none(), "one permanent: no two different targets, so {{4}}");
}

#[test]
fn a_spell_whose_gift_widens_its_target_is_offered_promised_at_what_makes_it_affordable() {
    let (mut g, promise, _, _) = board(&PROMISE, true);
    assert!(offered(&mut g, promise).is_some(), "promised, it may target the caster's Bear for {{1}} (CR 702.174m, 601.2f)");
    let (mut g, promise, _, _) = board(&PROMISE, false);
    assert!(offered(&mut g, promise).is_none(), "only the opponent's Bear: {{4}}");
}

#[test]
fn a_spell_dividing_among_its_targets_is_offered_for_what_its_targets_make_it_cost() {
    let (mut g, split, _, _) = board(&SPLIT, true);
    assert!(offered(&mut g, split).is_some(), "{{1}} divides among targets one of which is the caster's (CR 601.2d, 601.2f)");
    let (mut g, split, _, _) = board(&SPLIT, false);
    assert!(offered(&mut g, split).is_none(), "only the opponent's Bear: {{4}}");
}

#[test]
fn an_arcane_spell_is_offered_for_what_the_targets_of_a_card_spliced_onto_it_make_it_cost() {
    let (mut g, rite, _, _) = board(&RITE, true);
    assert!(offered(&mut g, rite).is_none(), "nothing to splice: {{4}}");
    put(&mut g, &GRAFT, YOU, Zone::hand(YOU));
    assert!(offered(&mut g, rite).is_none(), "spliced at the caster's Bear it costs {{2}} (CR 702.47a-c, 601.2f)");
    add_mana(&mut g, 1);
    assert!(offered(&mut g, rite).is_some(), "{{2}} casts it with the Graft spliced, aimed at the caster's Bear");
    let (mut g, rite, _, _) = board(&RITE, false);
    put(&mut g, &GRAFT, YOU, Zone::hand(YOU));
    add_mana(&mut g, 1);
    assert!(offered(&mut g, rite).is_none(), "only the opponent's Bear: {{4}}, or {{5}} spliced");
}

#[test]
fn a_spell_whose_cost_reads_its_kicker_is_offered_kicked_where_that_is_cheaper() {
    let (mut g, surge, _, theirs) = board(&SURGE, false);
    assert!(offered(&mut g, surge).is_none(), "kicked it costs {{2}}");
    add_mana(&mut g, 1);
    let option = offered(&mut g, surge).expect("{2} casts it kicked (CR 601.2b, 601.2f)");
    assert_eq!(option.target_candidates, vec![Target::Object(theirs)]);
}

#[test]
fn a_bestowed_aura_is_offered_for_the_creatures_it_could_be_paid_for_at() {
    let (mut g, wings, yours, _) = board(&WINGS, true);
    let options: Vec<ActionOption> = priority_options(&mut cx(&mut g), YOU).into_iter().filter(|o| o.source == Some(wings)).collect();
    assert_eq!(options.len(), 1, "only bestowed, for {{1}} at the caster's Bear (CR 702.103a, 601.2c, 601.2f)");
    assert_eq!(options[0].target_candidates, vec![Target::Object(yours)]);
    let (mut g, wings, _, _) = board(&WINGS, false);
    assert!(offered(&mut g, wings).is_none(), "only the opponent's Bear: {{4}}");
}

#[test]
fn a_modal_ability_is_offered_for_the_mode_and_target_that_make_it_affordable() {
    let (mut g, dial, _, _) = board(&DIAL, true);
    assert!(offered(&mut g, dial).is_some(), "{{1}} activates the creature mode at the controller's Bear (CR 602.2b, 601.2b-c)");
    let (mut g, dial, _, _) = board(&DIAL, false);
    assert!(offered(&mut g, dial).is_none(), "only the opponent's Bear: {{4}}");
}

#[test]
fn an_ability_of_x_targets_is_offered_for_the_x_whose_targets_make_it_affordable() {
    let (mut g, hive, _, _) = board(&HIVE, true);
    assert!(offered(&mut g, hive).is_some(), "X 1 at the controller's Bear costs {{1}} (CR 602.2b, 601.2b-c)");
    let (mut g, hive, _, _) = board(&HIVE, false);
    assert!(offered(&mut g, hive).is_none(), "X 0 costs {{3}}, and X 1 at the opponent's Bear {{4}}");
}

/// `n` more Bears of the opponent's.
fn their_bears(g: &mut Game, n: usize) {
    for _ in 0..n {
        put(g, &BEAR, THEM, Zone::BATTLEFIELD);
    }
}

#[test]
fn past_the_budget_an_action_whose_targets_change_its_cost_is_offered() {
    // Three of the opponent's creatures: every announcement is judged, and none costs less than {9}.
    let (mut g, bulk, _, _) = board(&BULK, false);
    their_bears(&mut g, 2);
    assert!(offered(&mut g, bulk).is_none(), "every choice of the opponent's creatures costs {{9}}");
    // Fifteen: more choices than the budget allows, each told apart by the change, so it is offered rather than withheld
    // (KI-108), and choosing them takes the cast back at the payment (CR 601.2h, 733.1).
    let (mut g, bulk, _, _) = board(&BULK, false);
    their_bears(&mut g, 14);
    assert!(offered(&mut g, bulk).is_some(), "past ANNOUNCEMENT_BUDGET the search answers that it might be paid for");
    // The same for an ability (CR 602.2b).
    let (mut g, grid, _, _) = board(&GRID, false);
    their_bears(&mut g, 2);
    assert!(offered(&mut g, grid).is_none(), "every choice of the opponent's creatures costs {{9}}");
    let (mut g, grid, _, _) = board(&GRID, false);
    their_bears(&mut g, 14);
    assert!(offered(&mut g, grid).is_some(), "past ANNOUNCEMENT_BUDGET it is offered");
}

#[test]
fn a_spell_that_counts_its_targets_is_judged_by_how_many_on_a_board_of_many() {
    let (mut g, volley, _, _) = board(&VOLLEY, false);
    their_bears(&mut g, 19);
    g.state.update_player(YOU, |p| p.pool = Default::default());
    assert!(offered(&mut g, volley).is_none(), "no mana: {{1}} at the least, whatever is chosen among twenty creatures");
    add_mana(&mut g, 1);
    assert!(offered(&mut g, volley).is_some(), "{{1}} casts it at one creature or none");
}

#[test]
fn a_modal_spell_is_offered_for_the_modes_the_board_allows_beyond_its_printed_number() {
    let (mut g, pairs, _, _) = board(&PAIRS, true);
    g.state.update_player(YOU, |p| p.pool = Default::default());
    assert!(offered(&mut g, pairs).is_some(), "both modes, two targets: {{0}} (CR 700.2, 601.2f)");
    let mut g = game();
    put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let pairs = put(&mut g, &PAIRS, YOU, Zone::hand(YOU));
    add_mana(&mut g, 3);
    assert!(offered(&mut g, pairs).is_none(), "no creature of the caster's: one mode, one target, {{4}}");
}

#[test]
fn a_spell_whose_x_is_paid_in_life_is_offered_for_the_x_whose_targets_make_it_affordable() {
    let (mut g, covenant, _, _) = board(&COVENANT, true);
    assert!(offered(&mut g, covenant).is_some(), "X 1, paid in life, at the caster's Bear: {{1}} (CR 601.2b-d, 601.2f)");
    let (mut g, covenant, _, _) = board(&COVENANT, false);
    assert!(offered(&mut g, covenant).is_none(), "only the opponent's Bear: {{4}}");
}
