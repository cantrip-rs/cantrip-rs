//! An Aura or Equipment put onto the battlefield attached to an object the effect names (`Game::put_onto_battlefield_attached`):
//! the effect names what an Aura enchants, so nothing is chosen (CR 303.4f), and an Aura the named object is not a legal one for stays
//! where it is (CR 303.4i); an Equipment enters attached to a creature it could equip and unattached to anything else (CR 301.5e).
//! Over test cards shaped like Eagle's Rescue's "return this card from your graveyard to the battlefield attached to target
//! creature". Several enter together (`Game::put_all_onto_battlefield_attached`), each on the host named for it, as Storm Herald's
//! "return any number of Aura cards from your graveyard to the battlefield attached to creatures you control" puts them: one
//! event (CR 101.4, 603.6a), each host judged before any of them moves.

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const OPPONENT: PlayerId = PlayerId(1);

static LAND: CardDef = CardDef { name: "Attached Entry Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static BEAR: CardDef = CardDef {
    name: "Attached Entry Bear",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Bear]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn creature_on_the_battlefield(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// An Aura with "enchant creature".
static AURA: CardDef = CardDef {
    name: "Attached Entry Aura",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(target(creature_on_the_battlefield)),
    ..CardDef::BLANK
};

static EQUIPMENT: CardDef = CardDef {
    name: "Attached Entry Equipment",
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    ..CardDef::BLANK
};

/// "Enchanted creature has protection from mana value 3 or greater": a mana value 0 Aura.
const WARDING_STATICS: &[StaticDef] = &[static_ability(
    Layer::Ability,
    cantrip_mechanics::attach::attached_object,
    from_mana_value_three_or_more,
    "Enchanted creature has protection from mana value 3 or greater",
)];

fn from_mana_value_three_or_more(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.protection_mana_value = Some(3);
}

static WARDING_AURA: CardDef = CardDef {
    name: "Attached Entry Warding Aura",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(target(creature_on_the_battlefield)),
    statics: WARDING_STATICS,
    ..CardDef::BLANK
};

/// An Aura with "enchant creature" and mana value 3, which the Warding Aura's creature has protection from.
static COSTLY_AURA: CardDef = CardDef {
    name: "Attached Entry Costly Aura",
    cost: ManaCost { generic: 3, ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(target(creature_on_the_battlefield)),
    ..CardDef::BLANK
};

fn an_aura_enters(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Battlefield) && q.chars(*new).has_subtype(Subtype::Aura) => Fired::PLAIN,
        _ => None,
    }
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// "Whenever one or more Auras enter, do nothing" (CR 603.2c).
static WATCHER: CardDef = CardDef {
    name: "Attached Entry Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(cantrip_api::when(EventMask::ZONE_CHANGE, an_aura_enters), nothing, "Whenever one or more Auras enter, do nothing.")
        .once_per_batch()],
    ..CardDef::BLANK
};

fn creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_on_the_battlefield(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}

/// An Aura with "enchant creature you control": who "you" is, is the player it enters under.
static YOUR_AURA: CardDef = CardDef {
    name: "Attached Entry Your Aura",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(target(creature_you_control)),
    ..CardDef::BLANK
};

fn aura(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.has_subtype(Subtype::Aura)
}

fn add_creature(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.types = c.types.with(CardType::Creature);
}

/// "Auras are creatures": on the battlefield only, so an Aura card in a graveyard is none, and would be one there (CR 303.4d).
static ANIMATOR: CardDef = CardDef {
    name: "Attached Entry Animator",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Type, aura, add_creature, "Auras are creatures.")],
    ..CardDef::BLANK
};

fn equipment(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.has_subtype(Subtype::Equipment)
}

/// "Equipment are creatures", on the battlefield only: an Equipment that's a creature can't equip (CR 301.5c).
static EQUIPMENT_ANIMATOR: CardDef = CardDef {
    name: "Attached Entry Equipment Animator",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Type, equipment, add_creature, "Equipment are creatures.")],
    ..CardDef::BLANK
};

/// A creature with hexproof, which an Aura that does not target may enchant.
static HEXPROOF_BEAR: CardDef = CardDef {
    name: "Attached Entry Hexproof Bear",
    types: TypeSet::of(&[CardType::Creature]),
    keywords: Keywords::of(&[Keyword::Hexproof]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A Treasure-like artifact that is no creature: an Equipment cannot equip it.
static ARTIFACT: CardDef = CardDef { name: "Attached Entry Artifact", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&LAND, &BEAR, &AURA, &EQUIPMENT, &ARTIFACT, &WARDING_AURA, &COSTLY_AURA, &WATCHER, &YOUR_AURA, &ANIMATOR, &HEXPROOF_BEAR, &EQUIPMENT_ANIMATOR])));
    let deck = || vec![table.id_of(&LAND); 20];
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
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(zone, id);
    g.state.see_statics();
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn put_attached(g: &mut Game, card: ObjectId, host: ObjectId) -> Option<ObjectId> {
    cantrip_api::Game::put_onto_battlefield_attached(&mut cx(g), card, YOU, host).expect("nothing is asked: the host is named")
}

fn put_all_attached(g: &mut Game, entries: &[(ObjectId, ObjectId)]) -> Vec<Option<ObjectId>> {
    let entries: Vec<(ObjectId, PlayerId, ObjectId)> = entries.iter().map(|&(card, host)| (card, YOU, host)).collect();
    cantrip_api::Game::put_all_onto_battlefield_attached(&mut cx(g), &entries).expect("nothing is asked: the hosts are named")
}

/// The state-based actions performed in a settling frame of the game's own, as the rules perform them.
fn state_based_actions(g: &mut Game) {
    g.state.frame_push(Frame::new(FrameKind::Settle));
    let frame = g.state.agenda.len() - 1;
    let mut cx = Cx { g, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    crate::settle::state_based_actions_alone(&mut cx).expect("nothing is asked");
}

/// The permanents that entered the battlefield, in the order the history records them.
fn entered(g: &Game) -> Vec<ObjectId> {
    g.log
        .iter()
        .filter_map(|r| match r.event {
            Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Battlefield) => Some(new),
            _ => None,
        })
        .collect()
}

/// With two creatures it could enchant, the Aura enters attached to the one the effect names: nobody is asked, and it is attached as
/// it enters (`Event::Attached`), not moved onto it afterwards.
#[test]
fn an_aura_enters_attached_to_the_host_named_though_another_is_legal() {
    let mut g = game();
    let first = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let named = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &AURA, YOU, Zone::graveyard(YOU));
    let aura = put_attached(&mut g, card, named).expect("it entered");
    assert!(g.state.obj(aura).zone.is(ZoneKind::Battlefield));
    assert_eq!(g.state.obj(aura).attached_to, Some(named));
    assert_ne!(g.state.obj(aura).attached_to, Some(first));
    assert!(g.log.iter().any(|r| matches!(r.event, Event::Attached { attachment, to: Target::Object(host) } if attachment == aura && host == named)));
}

/// A land is no creature, so "enchant creature" does not allow it: the Aura stays in the graveyard, a card that did not move
/// (CR 303.4i).
#[test]
fn an_aura_stays_in_the_graveyard_when_the_host_is_not_a_legal_object() {
    let mut g = game();
    put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let land = put(&mut g, &LAND, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &AURA, YOU, Zone::graveyard(YOU));
    assert_eq!(put_attached(&mut g, card, land), None);
    assert!(g.state.obj(card).zone.is(ZoneKind::Graveyard));
    assert!(g.state.obj(card).became.is_none(), "the card did not move");
}

/// An Equipment enters attached to a creature it could equip (CR 301.5e), and unattached to an artifact it could not: it enters
/// all the same.
#[test]
fn an_equipment_enters_attached_to_a_creature_and_unattached_to_anything_else() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let artifact = put(&mut g, &ARTIFACT, YOU, Zone::BATTLEFIELD);

    let card = put(&mut g, &EQUIPMENT, YOU, Zone::graveyard(YOU));
    let equipment = put_attached(&mut g, card, bear).expect("it entered");
    assert_eq!(g.state.obj(equipment).attached_to, Some(bear));

    let other = put(&mut g, &EQUIPMENT, YOU, Zone::graveyard(YOU));
    let loose = put_attached(&mut g, other, artifact).expect("it entered");
    assert!(g.state.obj(loose).zone.is(ZoneKind::Battlefield));
    assert_eq!(g.state.obj(loose).attached_to, None);
}

/// Two Auras returned together, each to the creature named for it: both enter in one move, each attached to its own host, and a
/// "whenever one or more Auras enter" ability triggers once (CR 101.4, 603.2c, 603.6a) - never once for each, as entering one at a
/// time would.
#[test]
fn two_auras_enter_together_each_attached_to_its_own_host() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let (first, second) = (put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD), put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD));
    let (a, b) = (put(&mut g, &AURA, YOU, Zone::graveyard(YOU)), put(&mut g, &AURA, YOU, Zone::graveyard(YOU)));
    let new = put_all_attached(&mut g, &[(a, first), (b, second)]);
    let (Some(on_first), Some(on_second)) = (new[0], new[1]) else { panic!("both entered: {new:?}") };
    assert_eq!(g.state.obj(on_first).attached_to, Some(first));
    assert_eq!(g.state.obj(on_second).attached_to, Some(second));
    assert_eq!(entered(&g), vec![on_first, on_second]);
    assert_eq!(g.state.triggers.iter().filter(|t| t.source == watcher).count(), 1, "one batch, one trigger");
}

/// The same two Auras returned one at a time are two batches: the watcher triggers twice. What the test above tells apart.
#[test]
fn two_auras_returned_one_at_a_time_are_two_batches() {
    let mut g = game();
    let watcher = put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let (first, second) = (put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD), put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD));
    let (a, b) = (put(&mut g, &AURA, YOU, Zone::graveyard(YOU)), put(&mut g, &AURA, YOU, Zone::graveyard(YOU)));
    put_attached(&mut g, a, first).expect("it entered");
    put_attached(&mut g, b, second).expect("it entered");
    assert_eq!(g.state.triggers.iter().filter(|t| t.source == watcher).count(), 2);
}

/// One Aura is named a land, which "enchant creature" does not allow: it stays in the graveyard, a card that did not move
/// (CR 303.4i), and the other enters attached all the same.
#[test]
fn an_aura_with_no_legal_host_stays_while_the_other_enters() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let land = put(&mut g, &LAND, YOU, Zone::BATTLEFIELD);
    let (stays, enters) = (put(&mut g, &AURA, YOU, Zone::graveyard(YOU)), put(&mut g, &AURA, YOU, Zone::graveyard(YOU)));
    let new = put_all_attached(&mut g, &[(stays, land), (enters, bear)]);
    assert_eq!(new[0], None);
    assert!(g.state.obj(stays).zone.is(ZoneKind::Graveyard) && g.state.obj(stays).became.is_none(), "the card did not move");
    let aura = new[1].expect("it entered");
    assert_eq!(g.state.obj(aura).attached_to, Some(bear));
}

/// The Warding Aura and the Costly Aura are returned together onto one creature. Each host is judged as the game is before
/// either moves (CR 303.4i, 101.4), when the creature has no protection, so both enter attached; once there, the Warding Aura's
/// protection from mana value 3 or greater makes the creature an illegal object for the Costly Aura, which the state-based
/// actions put into its owner's graveyard (CR 704.5m, 702.16c). The Warding Aura, judged by the same check, stays.
#[test]
fn an_aura_entering_with_one_that_gives_its_host_protection_from_it_enters_then_falls_off() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let (warding, costly) = (put(&mut g, &WARDING_AURA, YOU, Zone::graveyard(YOU)), put(&mut g, &COSTLY_AURA, YOU, Zone::graveyard(YOU)));
    let new = put_all_attached(&mut g, &[(warding, bear), (costly, bear)]);
    let (Some(warding), Some(costly)) = (new[0], new[1]) else { panic!("both entered: {new:?}") };
    assert_eq!(g.state.obj(costly).attached_to, Some(bear), "it entered attached");
    assert_eq!(cx(&mut g).chars(bear).protection_mana_value, Some(3));
    state_based_actions(&mut g);
    assert!(g.state.obj(costly).became.is_some_and(|c| g.state.obj(c).zone.is(ZoneKind::Graveyard)), "it fell off");
    assert_eq!(g.state.obj(warding).attached_to, Some(bear));
    assert!(g.state.obj(warding).zone.is(ZoneKind::Battlefield));
}

/// The creature already has that protection: the Costly Aura, judged before the move, has no legal host and stays in the
/// graveyard (CR 303.4i, 702.16c) - it never enters to fall off.
#[test]
fn an_aura_its_host_already_has_protection_from_stays() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let warding = put(&mut g, &WARDING_AURA, YOU, Zone::graveyard(YOU));
    put_attached(&mut g, warding, bear).expect("it entered");
    let costly = put(&mut g, &COSTLY_AURA, YOU, Zone::graveyard(YOU));
    assert_eq!(put_all_attached(&mut g, &[(costly, bear)]), vec![None]);
    assert!(g.state.obj(costly).zone.is(ZoneKind::Graveyard) && g.state.obj(costly).became.is_none());
}

/// An Aura named onto a creature card entering in the same move is not attached to it: a host is one there before the move
/// (CR 303.4i), so the Aura stays where it is while the creature card enters, and an Equipment named onto it enters unattached
/// (CR 301.5e).
#[test]
fn an_aura_cannot_enchant_an_object_entering_with_it() {
    let mut g = game();
    let creature = put(&mut g, &BEAR, YOU, Zone::graveyard(YOU));
    let card = put(&mut g, &AURA, YOU, Zone::graveyard(YOU));
    let equipment = put(&mut g, &EQUIPMENT, YOU, Zone::graveyard(YOU));
    let new = put_all_attached(&mut g, &[(creature, creature), (card, creature), (equipment, creature)]);
    let bear = new[0].expect("the creature card entered, attached to nothing");
    assert_eq!(g.state.obj(bear).attached_to, None);
    assert_eq!(new[1], None);
    assert!(g.state.obj(card).zone.is(ZoneKind::Graveyard));
    let equipment = new[2].expect("an Equipment enters unattached");
    assert_eq!(g.state.obj(equipment).attached_to, None);
}

/// Whether `card` could be put onto the battlefield under `controller` attached to `host` (`Game::could_enter_attached`), the
/// question a card asks before its player chooses - checked against the move itself, made on a copy of the game: the move
/// puts it there attached to `host` exactly when the question said it could. Asking moves nothing and records nothing: the
/// position after it is the one before (`GameState`'s `PartialEq`: every object, zone order, timestamp counter, effect and
/// pending trigger), and the history has no new event.
fn offered(g: &mut Game, card: ObjectId, controller: PlayerId, host: ObjectId) -> bool {
    let (before, events) = (g.state.clone(), g.log.len());
    let offered = cantrip_api::Game::could_enter_attached(&mut cx(g), card, controller, host);
    assert!(g.state == before, "asking left the position as it was");
    assert_eq!(g.log.len(), events, "asking recorded no event");
    let mut moved = g.clone();
    let new = cantrip_api::Game::put_onto_battlefield_attached(&mut cx(&mut moved), card, controller, host).expect("nothing is asked: the host is named");
    let attached = new.is_some_and(|n| moved.state.obj(n).attached_to == Some(host));
    assert_eq!(offered, attached, "the question and the move agree");
    offered
}

/// An Aura card in a graveyard can be put onto the battlefield attached to a creature its enchant ability allows and to no
/// land (CR 303.4f, 303.4i, 702.5a). `can_attach`, which asks of an attachment on the battlefield, says no to both.
#[test]
fn an_aura_card_in_a_graveyard_could_enter_attached_to_what_it_could_enchant() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let land = put(&mut g, &LAND, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &AURA, YOU, Zone::graveyard(YOU));
    assert!(offered(&mut g, card, YOU, bear));
    assert!(!offered(&mut g, card, YOU, land));
    assert!(!cantrip_api::Game::can_attach(&cx(&mut g), card, bear), "can_attach is for an attachment on the battlefield");
}

/// "Enchant creature you control" is read on the Aura as it would exist on the battlefield, under the player it enters under
/// (CR 614.12, 303.4f): an opponent's card returned under your control can enchant your creature - though in its owner's
/// graveyard it is theirs - and the same card entering under its owner could not.
#[test]
fn the_enchant_ability_is_read_on_the_aura_as_it_would_exist_on_the_battlefield() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &YOUR_AURA, OPPONENT, Zone::graveyard(OPPONENT));
    assert_eq!(cantrip_api::Query::controller(&cx(&mut g), card), OPPONENT);
    assert!(offered(&mut g, card, YOU, bear));
    assert!(!offered(&mut g, card, OPPONENT, bear));
}

/// "Auras are creatures" applies on the battlefield: the Aura card in the graveyard is no creature, but it would be one there, and
/// an Aura that's also a creature can enchant nothing (CR 303.4d, 614.12) - so it could enter attached to nothing.
#[test]
fn an_aura_that_would_be_a_creature_could_enter_attached_to_nothing() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &AURA, YOU, Zone::graveyard(YOU));
    put(&mut g, &ANIMATOR, YOU, Zone::BATTLEFIELD);
    assert!(!cx(&mut g).chars(card).is(CardType::Creature), "no creature in the graveyard");
    assert!(!offered(&mut g, card, YOU, bear));
}

/// A creature with protection from the Aura can't be enchanted by it (CR 702.16c): the Costly Aura could not enter attached to a
/// creature the Warding Aura gives protection from mana value 3 or greater, and the Aura with mana value 0 could. Hexproof
/// does not matter, as the Aura does not target (CR 303.4f).
#[test]
fn protection_keeps_an_aura_from_entering_attached_and_hexproof_does_not() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let warding = put(&mut g, &WARDING_AURA, YOU, Zone::graveyard(YOU));
    put_attached(&mut g, warding, bear).expect("it entered");
    let (costly, cheap) = (put(&mut g, &COSTLY_AURA, YOU, Zone::graveyard(YOU)), put(&mut g, &AURA, YOU, Zone::graveyard(YOU)));
    assert!(!offered(&mut g, costly, YOU, bear));
    assert!(offered(&mut g, cheap, YOU, bear));
    let hexproof = put(&mut g, &HEXPROOF_BEAR, OPPONENT, Zone::BATTLEFIELD);
    assert!(offered(&mut g, costly, YOU, hexproof));
}

/// An Aura can't enchant itself (CR 303.4d): the card named as its own host could not enter, nor could one named a creature
/// that has since left the battlefield, a new object (CR 400.7).
#[test]
fn an_aura_could_not_enter_attached_to_itself_or_to_an_object_gone() {
    let mut g = game();
    let card = put(&mut g, &AURA, YOU, Zone::graveyard(YOU));
    assert!(!offered(&mut g, card, YOU, card));
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let gone = crate::ops::move_to(&mut cx(&mut g), bear, Zone::graveyard(YOU));
    assert_ne!(gone, bear);
    assert!(!offered(&mut g, card, YOU, bear));
}

/// The question's moment on the battlefield (`ops::as_it_would_exist`) is journalled and taken back, so the journal still undoes
/// past it: an Aura card in the middle of a graveyard is asked about, the move then made, and reverting to a mark from before
/// the question gives the position before it, the card in its place in its graveyard (CR 404.2: order kept); asking and moving
/// again reaches the same position as the first time.
#[test]
fn asking_is_taken_back_with_the_journal() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    put(&mut g, &LAND, YOU, Zone::graveyard(YOU));
    let card = put(&mut g, &AURA, YOU, Zone::graveyard(YOU));
    put(&mut g, &LAND, YOU, Zone::graveyard(YOU));
    let (before, mark) = (g.state.clone(), g.state.mark());
    assert!(offered(&mut g, card, YOU, bear));
    let new = put_attached(&mut g, card, bear).expect("it entered");
    assert_eq!(g.state.obj(new).attached_to, Some(bear));
    let after = g.state.clone();
    g.state.revert(mark);
    assert!(g.state == before, "undone to before the question");
    assert_eq!(g.state.zone(Zone::graveyard(YOU))[1], card, "in its place in the graveyard");
    assert!(offered(&mut g, card, YOU, bear));
    put_attached(&mut g, card, bear).expect("it entered");
    assert!(g.state == after, "the same position again");
}

/// An Equipment card could enter attached to a creature and not to an artifact that is no creature, where it enters unattached
/// (CR 301.5e).
#[test]
fn an_equipment_could_enter_attached_to_a_creature_only() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let artifact = put(&mut g, &ARTIFACT, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &EQUIPMENT, YOU, Zone::graveyard(YOU));
    assert!(offered(&mut g, card, YOU, bear));
    assert!(!offered(&mut g, card, YOU, artifact));
}

/// "Equipment are creatures" applies on the battlefield: the Equipment card in the graveyard is no creature, but it would be one
/// there, and an Equipment that's a creature can't equip (CR 301.5c, 614.12) - it enters unattached (CR 301.5e), never attached
/// for the state-based actions to take it off.
#[test]
fn an_equipment_that_would_be_a_creature_enters_unattached() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    put(&mut g, &EQUIPMENT_ANIMATOR, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &EQUIPMENT, YOU, Zone::graveyard(YOU));
    assert!(!cx(&mut g).chars(card).is(CardType::Creature), "no creature in the graveyard");
    assert!(!offered(&mut g, card, YOU, bear));
    let equipment = put_attached(&mut g, card, bear).expect("it entered");
    assert_eq!(g.state.obj(equipment).attached_to, None);
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::Attached { .. })), "it never became attached");
}
