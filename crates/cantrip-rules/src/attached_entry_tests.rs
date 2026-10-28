//! An Aura or Equipment put onto the battlefield attached to an object the effect names (`Game::put_onto_battlefield_attached`):
//! the effect names what an Aura enchants, so nothing is chosen (CR 303.4f), and an Aura the named object is not a legal one for stays
//! where it is (CR 303.4i); an Equipment enters attached to a creature it could equip and unattached to anything else (CR 301.5e).
//! Over test cards shaped like Eagle's Rescue's "return this card from your graveyard to the battlefield attached to target
//! creature".

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

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

/// A Treasure-like artifact that is no creature: an Equipment cannot equip it.
static ARTIFACT: CardDef = CardDef { name: "Attached Entry Artifact", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&LAND, &BEAR, &AURA, &EQUIPMENT, &ARTIFACT])));
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
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn put_attached(g: &mut Game, card: ObjectId, host: ObjectId) -> Option<ObjectId> {
    cantrip_api::Game::put_onto_battlefield_attached(&mut cx(g), card, YOU, host).expect("nothing is asked: the host is named")
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
