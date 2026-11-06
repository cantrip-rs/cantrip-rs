//! Mutate and merged permanents as the core does them (CR 702.140, 730), over test cards: a mutate card cast for its mutate
//! cost is a mutating creature spell that targets a non-Human creature with the same owner (CR 702.140a); with its target
//! illegal it enters as a creature (CR 702.140b), with it legal it merges over or under it (CR 702.140c), and the merged
//! permanent - the same object it was (CR 730.2c) - has its topmost component's characteristics and every ability of each
//! (CR 702.140e, 730.2a), a token only if its topmost component is (CR 730.2d), face up if its topmost is (CR 730.2e). It
//! leaves as one permanent, each component going where it goes (CR 730.3), and what finds what it became finds them all (CR
//! 730.3c). A copy of it has the merged characteristics (copiable, CR 730.2a) and is one object of one card.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Mutate Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn gain_one(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

fn gain_ten(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 10);
    Ok(())
}

fn this_mutates(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Mutated { permanent, .. } if *permanent == source => Fired::PLAIN,
        _ => None,
    }
}

fn this_dies(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. } if *old == source && from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard) => Fired::PLAIN,
        _ => None,
    }
}

/// "When this creature dies, you gain 1 life": a 2/2 Bear for {1}{G}, the creature mutated onto.
static BEAR: CardDef = CardDef {
    name: "Mutate Test Bear",
    cost: mana("{1}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Bear]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, this_dies), gain_one, "When this creature dies, you gain 1 life.")],
    text: "When this creature dies, you gain 1 life.",
    ..CardDef::BLANK
};

/// A 1/1 Human Soldier: no mutating spell may target it.
static SOLDIER: CardDef = CardDef {
    name: "Mutate Test Soldier",
    cost: mana("{W}"),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Human, Subtype::Soldier]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// "Mutate {1}{G} / Trample / Whenever this creature mutates, you gain 10 life": a 5/5 Beast for {5}{G}.
static BEAST: CardDef = CardDef {
    name: "Mutate Test Beast",
    cost: mana("{5}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Beast]),
    keywords: Keywords::of(&[Keyword::Trample]),
    power: Some(5),
    toughness: Some(5),
    ways: &[WayDef { kind: WayKind::Mutate, ..way(Cost::mana(mana("{1}{G}")), ZoneKind::Hand, "Mutate {1}{G}") }],
    abilities: &[triggered(when(EventMask::MUTATED, this_mutates), gain_ten, "Whenever this creature mutates, you gain 10 life.")],
    text: "Mutate {1}{G}\nTrample\nWhenever this creature mutates, you gain 10 life.",
    ..CardDef::BLANK
};

fn others_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    affected != source && q.controller(affected) == q.controller(source) && so_far.is(CardType::Creature)
}

fn plus_one(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.power = c.power.map(|p| p + 1);
    c.toughness = c.toughness.map(|t| t + 1);
}

/// "Mutate {G} / Flying / Other creatures you control get +1/+1": a 1/1 Elk for {3}{G}.
static ELK: CardDef = CardDef {
    name: "Mutate Test Elk",
    cost: mana("{3}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Elk]),
    keywords: Keywords::of(&[Keyword::Flying]),
    power: Some(1),
    toughness: Some(1),
    ways: &[WayDef { kind: WayKind::Mutate, ..way(Cost::mana(mana("{G}")), ZoneKind::Hand, "Mutate {G}") }],
    statics: &[static_ability(Layer::PtModify, others_you_control, plus_one, "Other creatures you control get +1/+1.")],
    text: "Mutate {G}\nFlying\nOther creatures you control get +1/+1.",
    ..CardDef::BLANK
};

/// A 1/1 Cat token.
static CAT: CardDef = CardDef {
    name: "Mutate Test Cat",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Cat]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// The back face of `WOLF_FRONT`, a 4/4 Wolf.
static WOLF_BACK: CardDef = CardDef {
    name: "Mutate Test Wolf",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Wolf]),
    power: Some(4),
    toughness: Some(4),
    ..CardDef::BLANK
};

/// A 2/2 Elf, the front of a double-faced card whose back is `WOLF_BACK`.
static WOLF_FRONT: CardDef = CardDef {
    name: "Mutate Test Pup",
    cost: mana("{1}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Elf]),
    power: Some(2),
    toughness: Some(2),
    back: Some(&WOLF_BACK),
    ..CardDef::BLANK
};

fn creature_on_battlefield(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

fn tuck(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(id) = item.target(0).object() {
        g.put_into_library(&[id], LibraryPlace::Top);
    }
    Ok(())
}

fn tuck_bottom(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(id) = item.target(0).object() {
        g.put_into_library(&[id], LibraryPlace::Bottom);
    }
    Ok(())
}

/// "Put target creature on the bottom of its owner's library."
static TUCK_BOTTOM: CardDef = CardDef {
    name: "Mutate Test Tuck Under",
    cost: mana("{U}"),
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(creature_on_battlefield)], tuck_bottom),
    text: "Put target creature on the bottom of its owner's library.",
    ..CardDef::BLANK
};

/// "Put target creature on top of its owner's library."
static TUCK: CardDef = CardDef {
    name: "Mutate Test Tuck",
    cost: mana("{U}"),
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(creature_on_battlefield)], tuck),
    text: "Put target creature on top of its owner's library.",
    ..CardDef::BLANK
};

fn a_card_to_a_graveyard(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.is_card && z.to.is(ZoneKind::Graveyard))
}

fn exile_instead(q: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::exile(z.to.player);
        z.exiled_with = Some(q.linked_to(asker));
    }
}

/// "If a card would be put into a graveyard from anywhere, exile it instead": a replacement effect for cards alone, which
/// says nothing of tokens (CR 730.3e), exiling each card with it (CR 607.2b, Valgavoth's shape), as every card of a merged
/// permanent it moves is (CR 730.3d).
static VOID: CardDef = CardDef {
    name: "Mutate Test Void",
    cost: mana("{2}{B}"),
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, a_card_to_a_graveyard, exile_instead, "exiles a card instead")],
    text: "If a card would be put into a graveyard from anywhere, exile it instead.",
    ..CardDef::BLANK
};

fn noop(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn itself(_: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected
}

/// "Has [its first delayed ability]" (`statics::grant_delayed_ability`'s shape): the card the ability is printed on, as the
/// layers name it.
fn grant_first_delayed(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let card = view.text_of(source);
    if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
        chars.abilities[slot] = Some(AbilityRef::delayed(card, 0));
    }
}

/// "This creature has '{T}: Do nothing.'": a 2/2 Squelcher giving itself its card's first delayed ability (Hexing
/// Squelcher's shape).
static SQUELCHER: CardDef = CardDef {
    name: "Mutate Test Squelcher",
    cost: mana("{1}{R}"),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Goblin]),
    power: Some(2),
    toughness: Some(2),
    statics: &[static_ability(Layer::Ability, itself, grant_first_delayed, "This creature has '{T}: Do nothing.'")],
    delayed: &[ability(TAP, noop, "{T}: Do nothing.")],
    ..CardDef::BLANK
};

/// "This creature gets +3/+3 as long as a card is exiled with it": its static ability reads its own exile (CR 607.2a,
/// Valgavoth's shape).
fn while_it_exiled(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && !view.exiled_with(view.linked_to(source)).is_empty()
}

fn plus_three(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.power = c.power.map(|p| p + 3);
    c.toughness = c.toughness.map(|t| t + 3);
}

/// A 1/1 Keeper with that ability.
static KEEPER: CardDef = CardDef {
    name: "Mutate Test Keeper",
    cost: mana("{1}{B}"),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Horror]),
    power: Some(1),
    toughness: Some(1),
    statics: &[static_ability(Layer::PtModify, while_it_exiled, plus_three, "This creature gets +3/+3 as long as a card is exiled with it.")],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// Your main phase with priority and an empty stack, every answer round-tripped through the game-log tree (`Game::verify`).
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &BEAR, &SOLDIER, &BEAST, &ELK, &CAT, &WOLF_FRONT, &TUCK, &TUCK_BOTTOM, &VOID, &SQUELCHER, &KEEPER])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 3;
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.verify = true;
    g
}

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let mut obj = Obj::new(g.cards.id_of(def), owner, zone);
    let timestamp = g.state.timestamp();
    obj.timestamp = timestamp;
    let id = g.state.push_obj(obj);
    g.state.zone_push(zone, id);
    if !def.statics.is_empty() {
        g.state.see_statics();
    }
    unask(g);
    id
}

/// The priority question asked again of the board as it now is.
fn unask(g: &mut Game) {
    if let Some(at) = g.state.agenda.len().checked_sub(1)
        && g.state.agenda[at].pending.as_ref().is_some_and(|r| matches!(r.kind, RequestKind::Priority { .. }))
    {
        g.state.update_frame(at, |f| f.pending = None);
    }
}

fn add(g: &mut Game, mana: Mana, n: u32) {
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, mana, n, source, None, ManaLasts::Step, false);
    if let Some(at) = g.state.agenda.len().checked_sub(1)
        && g.state.agenda[at].pending.as_ref().is_some_and(|r| matches!(r.kind, RequestKind::Priority { .. }))
    {
        g.state.update_frame(at, |f| f.pending = None);
    }
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn options(g: &mut Game) -> Vec<ActionOption> {
    match next(g).kind {
        RequestKind::Priority { options } => options,
        other => panic!("expected priority, got {other:?}"),
    }
}

/// The mutate cast of `card` (its first way), if offered.
fn mutate_option(g: &mut Game, card: ObjectId) -> Option<ActionOption> {
    options(g).into_iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.index == 1)
}

/// Casts `card` for its mutate cost targeting `target`, paying from the pool, and gives back the spell.
fn cast_mutating(g: &mut Game, card: ObjectId, target: ObjectId) -> ObjectId {
    let option = mutate_option(g, card).expect("the mutate cast is offered");
    let r = next(g);
    assert!(option.target_candidates.contains(&Target::Object(target)), "{target:?} may be targeted: {:?}", option.target_candidates);
    let action = Action { targets: vec![Target::Object(target)], ..Action::of(&option) };
    g.answer(r.id, r.player, Answer::Act(action)).expect("a legal answer");
    loop {
        let r = next(g);
        match &r.kind {
            RequestKind::ChooseTarget { candidates, .. } => {
                assert!(candidates.contains(&Target::Object(target)), "{target:?} may be targeted: {candidates:?}");
                g.answer(r.id, r.player, Answer::Targets(vec![Target::Object(target)])).expect("a legal target");
            }
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
                g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
            }
            _ => break,
        }
    }
    let spell = g.state.stack.last().expect("the spell is on the stack").object;
    assert_eq!(g.state.target(g.state.stack.last().unwrap(), 0), Target::Object(target));
    spell
}

/// Passes until the stack is empty, putting a merging spell `on_top` or under when asked (CR 702.140c).
fn resolve_all(g: &mut Game, on_top: bool) {
    g.run();
    let mut asked = false;
    while !g.state.stack.is_empty() || g.pending().is_some_and(|r| !matches!(r.kind, RequestKind::Priority { .. })) {
        let r = next(g);
        match &r.kind {
            RequestKind::Priority { .. } => g.answer(r.id, r.player, Answer::Pass).expect("a legal answer"),
            RequestKind::ChooseModes { modes, .. } if modes.first() == Some(&Label::Mutate { on_top: true }) => {
                asked = true;
                g.answer(r.id, r.player, Answer::Modes(vec![if on_top { 0 } else { 1 }])).expect("a legal answer")
            }
            // The owner orders a merged permanent's cards put into their library (CR 730.3a): here, as they are listed, reversed.
            RequestKind::DistributeCards { cards, destinations, .. } if destinations.first().is_some_and(|d| d.kind == DestinationKind::LibraryTop) => {
                LIBRARY_ASKED.with(|a| a.set(a.get() + 1));
                let order: Vec<ObjectId> = if REVERSE_LIBRARY.with(|r| r.get()) { cards.iter().rev().copied().collect() } else { cards.clone() };
                g.answer(r.id, r.player, Answer::Placements { placements: vec![order], shuffle: false }).expect("a legal order")
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    g.run();
    let _ = asked;
}

/// After a change made from outside the rules' frames: priority asked again, and everything resolved.
fn settle(g: &mut Game) {
    unask(g);
    resolve_all(g, true);
}

thread_local! {
    /// Whether the owner's order of a merged permanent's cards put into a library (CR 730.3a) reverses them as listed.
    static REVERSE_LIBRARY: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    /// How many times that order was asked.
    static LIBRARY_ASKED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// A Bear on your battlefield since an earlier turn, and a Beast in your hand with {1}{G} to mutate it.
fn bear_and_beast() -> (Game, ObjectId, ObjectId) {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    g.state.update_obj(bear, |o| o.controlled_since = 1);
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    (g, bear, beast)
}

/// CR 702.140a: the mutating creature spell targets a non-Human creature with the same owner as it - not a Human, not a
/// creature another player owns - and is offered only where it has one.
#[test]
fn a_mutating_spell_targets_a_non_human_creature_its_owner_owns() {
    let mut g = game();
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    put(&mut g, &SOLDIER, YOU, Zone::BATTLEFIELD);
    put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    add(&mut g, Mana::G, 2);
    assert!(mutate_option(&mut g, beast).is_none(), "a Human and an opponent's Bear are no targets");
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let spell = cast_mutating(&mut g, beast, bear);
    assert!(g.chars(spell).is(CardType::Creature), "a mutating creature spell is a creature spell");
    assert_eq!(g.state.player(YOU).pool.total(), 0, "it cost its mutate cost, {{1}}{{G}}");
}

/// CR 702.140c, 730.2a, 702.140e, 730.2c: put on top, the merged permanent is the Beast with the Bear's abilities as well, the
/// Bear's object - not summoning sick, still tapped and with its counters - and "whenever this creature mutates" triggers.
#[test]
fn merged_on_top_it_is_the_mutating_card_with_every_ability_and_the_same_object() {
    let (mut g, bear, beast) = bear_and_beast();
    g.state.update_obj(bear, |o| o.tapped = true);
    crate::counters::put(&mut cx(&mut g), bear, CounterKind::PlusOne, 1, YOU, crate::counters::How::Effect);
    let spell = cast_mutating(&mut g, beast, bear);
    let life = g.state.player(YOU).life;
    resolve_all(&mut g, true);
    assert!(g.state.obj(bear).zone.is(ZoneKind::Battlefield) && g.state.obj(bear).became.is_none(), "the same object");
    assert_eq!(g.state.obj(spell).became, Some(bear), "the spell is followed to the merged permanent (CR 702.140f)");
    assert_eq!(g.state.zone(Zone::BATTLEFIELD).len(), 1, "one permanent");
    let c = g.chars(bear);
    assert_eq!(c.name, "Mutate Test Beast");
    assert_eq!((c.power, c.toughness), (Some(6), Some(6)), "the Beast's 5/5 and its +1/+1 counter");
    assert!(c.has(Keyword::Trample));
    assert_eq!(c.abilities().count(), 2, "the Beast's trigger and the Bear's");
    assert!(g.state.obj(bear).tapped, "still tapped");
    assert_eq!(g.state.obj(bear).controlled_since, 1, "it hasn't just come under its controller's control (CR 730.2c)");
    assert_eq!(g.state.player(YOU).life, life + 10, "it mutated (CR 702.140d)");
    assert_eq!(g.history().all().iter().filter(|r| matches!(r.event, Event::ZoneChange { to, .. } if to.is(ZoneKind::Battlefield))).count(), 0, "nothing entered");
}

/// CR 702.140c, 702.140e: put under, it is the Bear with the Beast's abilities as well: trample, and the trigger, which
/// triggers though its card is under (the release notes).
#[test]
fn merged_under_it_is_the_creature_with_the_mutating_cards_abilities() {
    let (mut g, bear, beast) = bear_and_beast();
    cast_mutating(&mut g, beast, bear);
    let life = g.state.player(YOU).life;
    resolve_all(&mut g, false);
    let c = g.chars(bear);
    assert_eq!(c.name, "Mutate Test Bear");
    assert_eq!((c.power, c.toughness), (Some(2), Some(2)));
    assert!(c.has(Keyword::Trample), "the Beast's keyword");
    assert_eq!(c.abilities().count(), 2);
    assert_eq!(g.state.player(YOU).life, life + 10, "the Beast's trigger, from under it");
    assert_eq!(cantrip_api::Query::mana_value(&g, bear), 2, "the Bear's mana value");
}

/// CR 702.140b, 608.3b: its target gone, the spell is no longer mutating and enters as the creature spell it is; nothing
/// mutated.
#[test]
fn with_its_target_illegal_it_enters_as_a_creature() {
    let (mut g, bear, beast) = bear_and_beast();
    let spell = cast_mutating(&mut g, beast, bear);
    let life = g.state.player(YOU).life;
    cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[bear]);
    settle(&mut g);
    let permanent = g.state.obj(spell).became.expect("it resolved");
    assert!(g.state.obj(permanent).zone.is(ZoneKind::Battlefield));
    assert!(g.state.merge_of(permanent).is_none());
    assert_eq!(g.chars(permanent).name, "Mutate Test Beast");
    assert!(!g.cards.get(g.chars(permanent).card).mutating, "its card's own values, not the mutating face");
    assert_eq!(g.state.player(YOU).life, life + 1, "the Bear's dies trigger alone: it didn't mutate");
}

/// CR 702.140e, 613.1, 613.7: a component's static ability is the merged permanent's own; a second mutation adds a third
/// component and its trigger, and the Beast's trigger, already part of it, triggers again (CR 702.140d).
#[test]
fn every_component_gives_its_abilities_and_triggers_on_each_mutation() {
    let (mut g, bear, beast) = bear_and_beast();
    let other = put(&mut g, &CAT, YOU, Zone::BATTLEFIELD);
    cast_mutating(&mut g, beast, bear);
    resolve_all(&mut g, false);
    assert_eq!(g.chars(other).power, Some(1));
    let elk = put(&mut g, &ELK, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 1);
    cast_mutating(&mut g, elk, bear);
    let life = g.state.player(YOU).life;
    resolve_all(&mut g, false);
    assert_eq!(g.chars(other).power, Some(2), "the Elk's static ability, from under");
    let c = g.chars(bear);
    assert!(c.has(Keyword::Flying) && c.has(Keyword::Trample));
    assert_eq!(g.state.player(YOU).life, life + 10, "the Beast's trigger, for the second mutation");
    assert_eq!(g.state.merge_of(bear).map(|m| m.parts.len()), Some(2));
}

/// CR 730.3, 730.3c, 603.10a: it dies as one creature - its own dies trigger, looked back on, once - and each card is put
/// into the graveyard; what it became is all of them.
#[test]
fn a_merged_permanent_dies_as_one_and_each_card_goes_to_the_graveyard() {
    let (mut g, bear, beast) = bear_and_beast();
    cast_mutating(&mut g, beast, bear);
    resolve_all(&mut g, true);
    let life = g.state.player(YOU).life;
    let gone = cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[bear]);
    assert_eq!(gone.len(), 2, "both new objects");
    settle(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 1, "the Bear's dies trigger, once");
    let graveyard = g.state.zone(Zone::graveyard(YOU)).to_vec();
    assert_eq!(graveyard.len(), 2);
    let names: Vec<&str> = graveyard.iter().map(|&c| g.chars(c).name).collect();
    assert!(names.contains(&"Mutate Test Bear") && names.contains(&"Mutate Test Beast"), "{names:?}");
    assert_eq!(cantrip_api::Query::found_all_after_move(&g, bear).len(), 2, "CR 730.3c: both are found");
    let died = g.history().all().iter().filter(|r| matches!(r.event, Event::ZoneChange { from, .. } if from.is(ZoneKind::Battlefield))).count();
    assert_eq!(died, 1, "one permanent left the battlefield");
}

/// CR 730.3c: returned to its owner's hand, each card goes there; exiled, each is exiled.
#[test]
fn every_component_goes_to_the_zone_it_goes_to() {
    let (mut g, bear, beast) = bear_and_beast();
    cast_mutating(&mut g, beast, bear);
    resolve_all(&mut g, false);
    let back = cantrip_api::Game::return_to_hand(&mut cx(&mut g), &[bear]);
    assert_eq!(back.len(), 2);
    assert!(back.iter().all(|&c| g.state.obj(c).zone == Zone::hand(YOU) && g.state.zone(Zone::hand(YOU)).contains(&c)));
}

fn tuck_place(place: LibraryPlace) -> &'static CardDef {
    match place {
        LibraryPlace::Bottom => &TUCK_BOTTOM,
        _ => &TUCK,
    }
}

/// The merged Beast over Bear put into its owner's library at `place` by a resolving spell, the owner reversing the order
/// they arrived in or not: the names of the cards where they went, in library order.
fn tucked(place: LibraryPlace, reverse: bool) -> Vec<&'static str> {
    let (mut g, bear, beast) = bear_and_beast();
    cast_mutating(&mut g, beast, bear);
    resolve_all(&mut g, true);
    let below = g.state.zone(Zone::library(YOU)).len();
    let tuck = put(&mut g, tuck_place(place), YOU, Zone::hand(YOU));
    add(&mut g, Mana::U, 1);
    let option = options(&mut g).into_iter().find(|o| o.source == Some(tuck)).expect("the instant is offered");
    let r = next(&mut g);
    g.answer(r.id, r.player, Answer::Act(Action { targets: vec![Target::Object(bear)], ..Action::of(&option) })).expect("a legal cast");
    loop {
        let r = next(&mut g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { break };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
    REVERSE_LIBRARY.with(|r| r.set(reverse));
    LIBRARY_ASKED.with(|a| a.set(0));
    resolve_all(&mut g, true);
    REVERSE_LIBRARY.with(|r| r.set(true));
    assert_eq!(LIBRARY_ASKED.with(|a| a.get()), 1, "the owner was asked once (CR 730.3a)");
    let library = g.state.zone(Zone::library(YOU)).to_vec();
    assert_eq!(library.len(), below + 2);
    let ends = if place == LibraryPlace::Bottom { &library[below..] } else { &library[..2] };
    ends.iter().map(|&c| g.cards.get(g.state.obj(c).card).name).collect()
}

/// CR 730.3a: put on top of its owner's library, the owner orders its cards, and they stay in that order.
#[test]
fn its_owner_orders_its_cards_put_into_a_library() {
    // They arrive the Beast (the permanent's topmost) first, then the Bear on top of it, as the question lists them.
    assert_eq!(tucked(LibraryPlace::Top, true), vec!["Mutate Test Beast", "Mutate Test Bear"], "reversed, as the owner chose");
    assert_eq!(tucked(LibraryPlace::Top, false), vec!["Mutate Test Bear", "Mutate Test Beast"], "as listed, as the owner chose");
}

/// CR 730.3a: put on the bottom, the owner's order holds there too.
#[test]
fn its_owner_orders_its_cards_put_on_the_bottom_of_a_library() {
    assert_eq!(tucked(LibraryPlace::Bottom, true), vec!["Mutate Test Beast", "Mutate Test Bear"]);
    assert_eq!(tucked(LibraryPlace::Bottom, false), vec!["Mutate Test Bear", "Mutate Test Beast"]);
}

/// CR 730.2a, 707.2: a copy of a merged permanent copies its merged values - the topmost component's characteristics and
/// every component's abilities - and is one object of one card, which leaves as one.
#[test]
fn a_copy_of_a_merged_permanent_has_the_merged_values_and_is_one_object() {
    let (mut g, bear, beast) = bear_and_beast();
    cast_mutating(&mut g, beast, bear);
    resolve_all(&mut g, false);
    let values = cantrip_api::Query::copiable_values(&g, bear);
    let tokens = cantrip_api::Game::create_token_copies_of(&mut cx(&mut g), values, YOU, 1).expect("nothing is asked");
    let token = tokens[0];
    let c = g.chars(token);
    assert_eq!(c.name, "Mutate Test Bear");
    assert!(c.has(Keyword::Trample), "the Beast's ability is copiable");
    assert_eq!(c.abilities().count(), 2);
    assert!(g.state.merge_of(token).is_none(), "not a merged permanent");
    let gone = cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[token]);
    assert_eq!(gone.len(), 1, "one object");
}

/// CR 730.2d, 730.3: a card put under a token makes a token permanent; dying, the token goes to the graveyard (and ceases to
/// exist there, CR 704.5d) and the card stays there.
#[test]
fn a_merged_permanent_with_a_token_on_top_is_a_token() {
    let mut g = game();
    let cat_card = g.cards.id_of(&CAT);
    let cat = cantrip_api::Game::create_tokens(&mut cx(&mut g), cat_card, YOU, 1, false).expect("nothing is asked")[0];
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    cast_mutating(&mut g, beast, cat);
    resolve_all(&mut g, false);
    assert!(g.state.obj(cat).token, "its topmost component is a token");
    assert_eq!(g.chars(cat).name, "Mutate Test Cat");
    assert!(g.chars(cat).has(Keyword::Trample));
    cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[cat]);
    settle(&mut g);
    let graveyard = g.state.zone(Zone::graveyard(YOU)).to_vec();
    assert_eq!(graveyard.len(), 1, "the token ceased to exist; the Beast card is there");
    assert_eq!(g.chars(graveyard[0]).name, "Mutate Test Beast");
    assert!(!g.state.obj(graveyard[0]).token);
}

/// CR 730.2a, 702.140c: a card put on top of a token makes a nontoken permanent of the card; its token component still goes
/// as a token.
#[test]
fn a_card_on_top_of_a_token_is_no_token() {
    let mut g = game();
    let cat_card = g.cards.id_of(&CAT);
    let cat = cantrip_api::Game::create_tokens(&mut cx(&mut g), cat_card, YOU, 1, false).expect("nothing is asked")[0];
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    cast_mutating(&mut g, beast, cat);
    resolve_all(&mut g, true);
    assert!(!g.state.obj(cat).token, "its topmost component is a card");
    let gone = cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[cat]);
    assert_eq!(gone.iter().filter(|&&c| g.state.obj(c).token).count(), 1, "the Cat token among them");
}

/// The game-log tree: going back above the merge and down again restores the position exactly (every answer here is also
/// round-tripped by `Game::verify`).
#[test]
fn the_game_tree_moves_across_a_merge() {
    let (mut g, bear, beast) = bear_and_beast();
    cast_mutating(&mut g, beast, bear);
    let before = g.at;
    let state = g.state.clone();
    resolve_all(&mut g, true);
    let after = g.at;
    let merged = g.state.clone();
    assert_eq!(g.chars(bear).name, "Mutate Test Beast");
    g.goto(before);
    assert!(g.state == state, "back above the merge");
    assert!(g.state.merge_of(bear).is_none());
    assert_eq!(g.chars(bear).name, "Mutate Test Bear");
    g.goto(after);
    assert!(g.state == merged, "down again");
    assert_eq!(g.chars(bear).name, "Mutate Test Beast");
}

/// CR 613.2a, 613.7, 730.2a: a copy effect on the permanent before it merged is applied before the merge; once it ends, the
/// merge applies over the permanent's own values (the release notes: "the characteristics it gains from mutating apply after
/// applying the copy effect").
#[test]
fn a_copy_effect_below_a_merge_ending_leaves_the_merge_over_the_permanents_own_values() {
    let (mut g, bear, beast) = bear_and_beast();
    let soldier = put(&mut g, &SOLDIER, YOU, Zone::BATTLEFIELD);
    let elk_values = Copiable::of(g.cards.id_of(&ELK));
    cantrip_api::Game::become_copy(&mut cx(&mut g), soldier, bear, elk_values, Duration::EndOfTurn);
    assert_eq!(g.chars(bear).name, "Mutate Test Elk");
    cast_mutating(&mut g, beast, bear);
    resolve_all(&mut g, false);
    let c = g.chars(bear);
    assert_eq!(c.name, "Mutate Test Elk", "under the merge, the copy effect's values are the permanent's");
    assert!(c.has(Keyword::Trample) && c.has(Keyword::Flying));
    // The copy effect ends.
    g.state.effects_retain(|e| !matches!(e.kind, EffectKind::RestoreCopy(_)));
    let c = g.chars(bear);
    assert_eq!(c.name, "Mutate Test Bear", "the merge applies over the Bear's own values");
    assert!(c.has(Keyword::Trample) && !c.has(Keyword::Flying));
    assert_eq!(c.abilities().count(), 2);
}

/// CR 730.2e, 708.9: a face-up card put on top of a face-down permanent makes a face-up permanent (not turned face up), whose
/// face-down component gives nothing; as it leaves, that card is revealed.
#[test]
fn a_card_on_top_of_a_face_down_permanent_is_face_up_and_the_face_down_card_gives_nothing() {
    let mut g = game();
    let card = put(&mut g, &BEAR, YOU, Zone::hand(YOU));
    let manifested = cantrip_api::Game::put_onto_battlefield_face_down(&mut cx(&mut g), card, YOU, FaceDown::Manifested, false)
        .expect("nothing is asked")
        .expect("it entered");
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    cast_mutating(&mut g, beast, manifested);
    resolve_all(&mut g, true);
    assert!(!g.state.obj(manifested).face_down, "face up, its topmost component being face up");
    assert!(!g.history().all().iter().any(|r| matches!(r.event, Event::TurnedFaceUp { .. })), "not turned face up");
    let c = g.chars(manifested);
    assert_eq!(c.name, "Mutate Test Beast");
    assert_eq!(c.abilities().count(), 1, "the face-down Bear gives no abilities");
    let gone = cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[manifested]);
    let bear_card = gone.iter().copied().find(|&c| g.state.obj(c).card == g.cards.id_of(&BEAR)).expect("the Bear card");
    assert!(g.history().all().iter().any(|r| matches!(r.event, Event::Revealed { card, .. } if card == bear_card)), "revealed as it moved");
}

/// CR 730.2i: a merged permanent is no double-faced permanent, but transforming it turns its double-faced component: under
/// the Beast, the Pup becomes the Wolf, and the permanent has the Wolf's values and the Beast's abilities.
#[test]
fn transforming_a_merged_permanent_turns_its_double_faced_component() {
    let mut g = game();
    let pup = put(&mut g, &WOLF_FRONT, YOU, Zone::BATTLEFIELD);
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    cast_mutating(&mut g, beast, pup);
    resolve_all(&mut g, false);
    assert_eq!(g.chars(pup).name, "Mutate Test Pup");
    cantrip_api::Game::transform(&mut cx(&mut g), pup).expect("nothing is asked");
    let c = g.chars(pup);
    assert_eq!(c.name, "Mutate Test Wolf");
    assert_eq!((c.power, c.toughness), (Some(4), Some(4)));
    assert!(c.has(Keyword::Trample));
    let values = cantrip_api::Query::copiable_values(&g, pup);
    assert_eq!(values.faces(), None, "a copy of it is single-faced (CR 730.2i)");
}

/// CR 730.3e: a replacement effect for a card put into a zone, saying nothing of tokens, applies to every component of a
/// merged permanent that isn't a token, its token components included: the Beast on top of the Cat token dies, and both
/// are exiled instead - none of them reaches the graveyard, and the Beast card is in exile.
#[test]
fn a_card_only_replacement_moves_every_component_of_a_nontoken_merged_permanent() {
    let mut g = game();
    let void = put(&mut g, &VOID, THEM, Zone::BATTLEFIELD);
    let cat_card = g.cards.id_of(&CAT);
    let cat = cantrip_api::Game::create_tokens(&mut cx(&mut g), cat_card, YOU, 1, false).expect("nothing is asked")[0];
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    cast_mutating(&mut g, beast, cat);
    resolve_all(&mut g, true);
    assert!(!g.state.obj(cat).token, "its topmost component is a card");
    let gone = cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[cat]);
    assert_eq!(gone.len(), 2, "both components moved");
    assert!(gone.iter().all(|&c| g.state.obj(c).zone.is(ZoneKind::Exile)), "the token component is exiled with the card");
    assert!(gone.iter().all(|&c| g.state.obj(c).exiled.is_some_and(|l| l.by == void)), "each exiled with the Void (CR 607.2b, 730.3d)");
    settle(&mut g);
    assert!(g.state.zone(Zone::graveyard(YOU)).is_empty(), "nothing was put into the graveyard");
    let exiled: Vec<&str> = g.state.zone(Zone::exile(YOU)).iter().map(|&c| g.chars(c).name).collect();
    assert_eq!(exiled, vec!["Mutate Test Beast"], "the Beast card is in exile; the Cat token ceased to exist there (CR 704.5d)");
}

/// CR 730.3e: a merged permanent that is a token, a card among its components, is put where it goes with its token
/// components, and its card components are moved by the replacement effect for cards alone: the Cat token over the Beast
/// dies, the token goes to the graveyard, and the Beast card is exiled instead.
#[test]
fn a_card_only_replacement_moves_only_the_card_components_of_a_token_merged_permanent() {
    let mut g = game();
    let void = put(&mut g, &VOID, THEM, Zone::BATTLEFIELD);
    let cat_card = g.cards.id_of(&CAT);
    let cat = cantrip_api::Game::create_tokens(&mut cx(&mut g), cat_card, YOU, 1, false).expect("nothing is asked")[0];
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    cast_mutating(&mut g, beast, cat);
    resolve_all(&mut g, false);
    assert!(g.state.obj(cat).token, "its topmost component is a token");
    let gone = cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[cat]);
    assert_eq!(gone.len(), 2, "both components moved");
    let token = gone.iter().copied().find(|&c| g.state.obj(c).token).expect("the token component");
    let card = gone.iter().copied().find(|&c| !g.state.obj(c).token).expect("the card component");
    assert!(g.state.obj(token).zone.is(ZoneKind::Graveyard), "the token is put into the graveyard: the permanent died");
    assert!(g.state.obj(card).zone.is(ZoneKind::Exile), "the card is exiled instead");
    assert!(g.state.obj(card).exiled.is_some_and(|l| l.by == void), "exiled with the Void (CR 607.2b)");
    assert_eq!(g.chars(card).name, "Mutate Test Beast");
    let died = g.history().all().iter().filter(|r| matches!(r.event, Event::ZoneChange { from, to, .. } if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard))).count();
    assert_eq!(died, 1, "one permanent died");
}

/// A static ability of a component under another that gives its card's delayed ability gives that card's, not the topmost
/// card's (CR 702.140e, 607.1): the layers name the card the ability is printed on (`LayerQuery::text_of`).
#[test]
fn a_component_under_another_gives_its_own_cards_delayed_ability() {
    let mut g = game();
    let squelcher = put(&mut g, &SQUELCHER, YOU, Zone::BATTLEFIELD);
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    cast_mutating(&mut g, beast, squelcher);
    resolve_all(&mut g, true);
    let c = g.chars(squelcher);
    assert_eq!(c.name, "Mutate Test Beast");
    let given = AbilityRef::delayed(g.cards.id_of(&SQUELCHER), 0);
    assert!(c.abilities.contains(&Some(given)), "the Squelcher's own delayed ability: {:?}", c.abilities);
    assert!(!c.abilities.contains(&Some(AbilityRef::delayed(g.cards.id_of(&BEAST), 0))), "never the Beast's, which has none");
}

/// A component's static ability under another reads what its own card exiled (CR 607.2a, 702.140e): Valgavoth's shape, its
/// exiled cards kept after a mutation on top of it.
#[test]
fn a_component_under_another_reads_its_own_exiled_cards() {
    let mut g = game();
    let keeper = put(&mut g, &KEEPER, YOU, Zone::BATTLEFIELD);
    let exiled = put(&mut g, &FILLER, THEM, Zone::exile(THEM));
    let by = Linked { object: keeper, text: g.cards.id_of(&KEEPER), acquired: Acquired::OWN };
    g.state.update_obj(exiled, |o| o.exiled = Some(ExileLink::new(by, Zone::graveyard(THEM), false)));
    unask(&mut g);
    assert_eq!(g.chars(keeper).power, Some(4));
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    cast_mutating(&mut g, beast, keeper);
    resolve_all(&mut g, true);
    assert_eq!(g.chars(keeper).power, Some(8), "the Beast's 5/5 and the Keeper's +3/+3 for its own exiled card");
}

/// CR 730.3c, 607.2a: exiled with an ability, every card the merged permanent became is linked to it, so "return the exiled
/// cards" returns them all (Brutal Cathar, Portable Hole).
#[test]
fn every_component_is_exiled_with_the_ability_that_exiled_it() {
    let (mut g, bear, beast) = bear_and_beast();
    cast_mutating(&mut g, beast, bear);
    resolve_all(&mut g, true);
    let soldier = put(&mut g, &SOLDIER, THEM, Zone::BATTLEFIELD);
    let by = Linked { object: soldier, text: g.cards.id_of(&SOLDIER), acquired: Acquired::OWN };
    let gone = cantrip_api::Game::exile_with(&mut cx(&mut g), &[bear], by, false);
    assert_eq!(gone.len(), 2);
    assert!(gone.iter().all(|&c| g.state.obj(c).exiled.is_some_and(|l| l.is_by(by))), "both linked");
    assert_eq!(cantrip_api::Query::exiled_with(&g, by).len(), 2);
}

/// CR 730.3, 700.4: a merged permanent's other cards come from the battlefield with it - a card put into a graveyard from
/// the battlefield (`Event::moved_from`) - though only the permanent's own move is the permanent leaving; a token-topped one's
/// card components are cards.
#[test]
fn the_other_components_are_put_into_the_graveyard_from_the_battlefield() {
    let mut g = game();
    let cat_card = g.cards.id_of(&CAT);
    let cat = cantrip_api::Game::create_tokens(&mut cx(&mut g), cat_card, YOU, 1, false).expect("nothing is asked")[0];
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    cast_mutating(&mut g, beast, cat);
    resolve_all(&mut g, false);
    cantrip_api::Game::sacrifice(&mut cx(&mut g), YOU, &[cat]);
    let moves: Vec<Event> = g.history().all().iter().map(|r| r.event).filter(|e| matches!(e, Event::ZoneChange { old, to, .. } if *old == cat && to.is(ZoneKind::Graveyard))).collect();
    assert_eq!(moves.len(), 2);
    assert!(moves.iter().all(|e| e.moved_from() == Some(Zone::BATTLEFIELD)), "each from the battlefield");
    let from_battlefield = moves.iter().filter(|e| matches!(e, Event::ZoneChange { from, .. } if from.is(ZoneKind::Battlefield))).count();
    assert_eq!(from_battlefield, 1, "one permanent left");
    let card_part = moves.iter().find_map(|e| match e { Event::ZoneChange { new, part: true, .. } => Some(*new), _ => None }).expect("the Beast came along");
    assert!(!g.state.obj(card_part).token, "a card, though the permanent was a token");
}

/// CR 730.2e, 613.2b, 708.2: a card put under a face-down permanent leaves it face down, a 2/2 with no abilities: its
/// face-down values apply over the merge (layer 1b over 1a), so it has none of the card's abilities and nothing it has
/// triggers on its mutating. Turned face up, it has them.
#[test]
fn a_card_under_a_face_down_permanent_gives_nothing_while_it_is_face_down() {
    let mut g = game();
    let card = put(&mut g, &BEAR, YOU, Zone::hand(YOU));
    let manifested = cantrip_api::Game::put_onto_battlefield_face_down(&mut cx(&mut g), card, YOU, FaceDown::Manifested, false)
        .expect("nothing is asked")
        .expect("it entered");
    let beast = put(&mut g, &BEAST, YOU, Zone::hand(YOU));
    add(&mut g, Mana::G, 2);
    cast_mutating(&mut g, beast, manifested);
    let life = g.state.player(YOU).life;
    resolve_all(&mut g, false);
    assert!(g.state.obj(manifested).face_down, "still face down");
    let c = g.chars(manifested);
    assert_eq!((c.power, c.toughness), (Some(2), Some(2)));
    let beast_card = g.cards.id_of(&BEAST);
    assert!(!c.has(Keyword::Trample) && !c.abilities().any(|a| a.card == beast_card), "none of the Beast's abilities while face down");
    assert_eq!(g.state.player(YOU).life, life, "its mutate trigger isn't an ability it has");
    crate::ops::turn_face_up(&mut cx(&mut g), manifested, YOU);
    let c = g.chars(manifested);
    assert_eq!(c.name, "Mutate Test Bear");
    assert!(c.has(Keyword::Trample), "face up, the Beast's abilities");
}
