//! A creature's base power (CR 613.4a, 613.4b; the rulings of Kutzil, Malamet Exemplar and Baird, Argivian Recruiter): its power
//! as layers 7a and 7b leave it, kept apart from what 7c and 7d do with it, on the battlefield and in the last-known row
//! of a creature that has left or that has dealt damage.

use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;
use cantrip_mechanics::statics::{all_creatures, base_pt, plus_pt};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static BEAR: CardDef = CardDef {
    name: "Base Power Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A 1/3, the creature of CR 613.4d's examples.
static WALL: CardDef = CardDef {
    name: "Base Power Wall",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(3),
    ..CardDef::BLANK
};

fn five_six(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = Some(5);
    chars.toughness = Some(6);
}

/// A */* whose characteristic-defining ability makes it 5/6 (CR 604.3, 613.4a).
static DEFINED: CardDef = CardDef {
    name: "Base Power Defined",
    types: TypeSet::of(&[CardType::Creature]),
    statics: &[characteristic_defining(Layer::PtDefine, five_six, "This creature's power is 5 and its toughness is 6.")],
    ..CardDef::BLANK
};

fn itself(_: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected
}

/// A 0/0 with a static ability giving it a bonus, which is no characteristic-defining ability: the rulings' case.
static GROWN: CardDef = CardDef {
    name: "Base Power Grown",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    statics: &[static_ability(Layer::PtModify, itself, plus_pt::<3, 3>, "This creature gets +3/+3.")],
    ..CardDef::BLANK
};

static ANTHEM: CardDef = CardDef {
    name: "Base Power Anthem",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::PtModify, all_creatures, plus_pt::<1, 1>, "Creatures get +1/+1.")],
    ..CardDef::BLANK
};

/// Another permanent's static ability that sets power and toughness (CR 613.4b): Humility's shape.
static SHRINK: CardDef = CardDef {
    name: "Base Power Shrink",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::PtSet, all_creatures, base_pt::<1, 1>, "Creatures have base power and toughness 1/1.")],
    ..CardDef::BLANK
};

/// A Vehicle's shape: an artifact that prints a 3/2 box.
static CART: CardDef = CardDef {
    name: "Base Power Cart",
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Vehicle]),
    power: Some(3),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&BEAR, &WALL, &DEFINED, &GROWN, &ANTHEM, &SHRINK, &CART])));
    Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, damage: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn enter(game: &mut Game, def: &'static CardDef) -> ObjectId {
    let mut obj = Obj::new(game.cards.id_of(def), YOU, Zone::BATTLEFIELD);
    obj.timestamp = game.state.timestamp();
    let id = game.state.push_obj(obj);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.see_statics();
    id
}

fn effect(game: &mut Game, on: ObjectId, kind: EffectKind) {
    let timestamp = game.state.timestamp();
    game.state.effect_push(cantrip_state::ContinuousEffect { source: on, controller: YOU, timestamp, affected: on, kind, duration: Duration::EndOfTurn, words: Words::NONE });
}

/// Its power, and its base power.
fn read(game: &Game, id: ObjectId) -> (Option<i32>, Option<i32>) {
    let c = cantrip_api::Query::chars(game, id);
    (c.power, c.base_power)
}

/// With nothing acting on it, a creature's printed power is its base power, and it is not above it.
#[test]
fn a_printed_power_is_the_base_power() {
    let mut g = game();
    let bear = enter(&mut g, &BEAR);
    assert_eq!(read(&g, bear), (Some(2), Some(2)));
    assert!(!cantrip_api::Query::chars(&g, bear).power_above_base());
}

/// The rulings: a characteristic-defining ability that sets power and toughness is taken into account (CR 613.4a).
#[test]
fn a_characteristic_defining_ability_defines_the_base_power() {
    let mut g = game();
    let defined = enter(&mut g, &DEFINED);
    assert_eq!(read(&g, defined), (Some(5), Some(5)));
    assert!(!cantrip_api::Query::chars(&g, defined).power_above_base());
    effect(&mut g, defined, EffectKind::Pump(1, 1));
    assert_eq!(read(&g, defined), (Some(6), Some(5)));
    assert!(cantrip_api::Query::chars(&g, defined).power_above_base());
}

/// The rulings: an ability giving a 0/0 a bonus is no characteristic-defining ability, and does not change its base power.
#[test]
fn a_bonus_on_a_zero_zero_is_above_its_base_power() {
    let mut g = game();
    let grown = enter(&mut g, &GROWN);
    assert_eq!(read(&g, grown), (Some(3), Some(0)));
    assert!(cantrip_api::Query::chars(&g, grown).power_above_base());
}

/// The rulings, CR 613.4b: an effect that sets power and toughness makes its value the base power, the latest in
/// timestamp order over the earlier and over a characteristic-defining ability, whenever a modification began.
#[test]
fn a_set_effect_sets_the_base_power() {
    let mut g = game();
    let bear = enter(&mut g, &BEAR);
    effect(&mut g, bear, EffectKind::Pump(2, 2));
    effect(&mut g, bear, EffectKind::SetPt(7, 7));
    assert_eq!(read(&g, bear), (Some(9), Some(7)));
    effect(&mut g, bear, EffectKind::SetPt(1, 1));
    assert_eq!(read(&g, bear), (Some(3), Some(1)));

    let defined = enter(&mut g, &DEFINED);
    effect(&mut g, defined, EffectKind::SetPt(0, 1));
    assert_eq!(read(&g, defined), (Some(0), Some(0)));
    assert!(!cantrip_api::Query::chars(&g, defined).power_above_base());
}

/// Layer 7b as another permanent's static ability applies it, and 7c as another's applies it: the layer pass's path.
#[test]
fn another_permanents_static_abilities_are_on_either_side_of_the_base_power() {
    let mut g = game();
    let bear = enter(&mut g, &BEAR);
    enter(&mut g, &ANTHEM);
    assert_eq!(read(&g, bear), (Some(3), Some(2)));
    enter(&mut g, &SHRINK);
    assert_eq!(read(&g, bear), (Some(2), Some(1)));
}

/// The rulings, CR 613.4c: an effect that modifies power without setting it is not in the base power.
#[test]
fn a_modification_is_not_in_the_base_power() {
    let mut g = game();
    let bear = enter(&mut g, &BEAR);
    effect(&mut g, bear, EffectKind::Pump(2, 0));
    assert_eq!(read(&g, bear), (Some(4), Some(2)));
    assert!(cantrip_api::Query::chars(&g, bear).power_above_base());
    effect(&mut g, bear, EffectKind::Pump(-3, 0));
    assert_eq!(read(&g, bear), (Some(1), Some(2)));
    assert!(!cantrip_api::Query::chars(&g, bear).power_above_base());
}

/// CR 613.4c: a +1/+1 counter modifies power; a -1/-1 counter too, however they stand beside each other.
#[test]
fn a_counter_is_not_in_the_base_power() {
    let mut g = game();
    let bear = enter(&mut g, &BEAR);
    cantrip_api::Game::add_counters_by(&mut cx(&mut g), YOU, bear, CounterKind::PlusOne, 2);
    assert_eq!(read(&g, bear), (Some(4), Some(2)));
    assert!(cantrip_api::Query::chars(&g, bear).power_above_base());
    cantrip_api::Game::add_counters_by(&mut cx(&mut g), YOU, bear, CounterKind::MinusOne, 2);
    assert_eq!(read(&g, bear), (Some(2), Some(2)));
    assert!(!cantrip_api::Query::chars(&g, bear).power_above_base());
}

/// CR 613.4d's first example: a 1/3 given +0/+1 and switched is 4/1. Its power is 4 and its base power is still 1, above it;
/// a creature whose toughness is the lower one switches below it.
#[test]
fn a_switch_does_not_change_the_base_power() {
    let mut g = game();
    let wall = enter(&mut g, &WALL);
    effect(&mut g, wall, EffectKind::Pump(0, 1));
    effect(&mut g, wall, EffectKind::SwitchPt);
    assert_eq!(read(&g, wall), (Some(4), Some(1)));
    assert!(cantrip_api::Query::chars(&g, wall).power_above_base());

    let bear = enter(&mut g, &BEAR);
    effect(&mut g, bear, EffectKind::Pump(0, -1));
    effect(&mut g, bear, EffectKind::SwitchPt);
    assert_eq!(read(&g, bear), (Some(1), Some(2)));
    assert!(!cantrip_api::Query::chars(&g, bear).power_above_base());
}

/// CR 208.3: a noncreature permanent has no power and so no base power; made a creature it has what it prints as its base
/// power, and the effect that modified it all along on top (CR 208.3a).
#[test]
fn a_noncreature_permanent_has_no_base_power_until_it_is_a_creature() {
    let mut g = game();
    let cart = enter(&mut g, &CART);
    assert_eq!(read(&g, cart), (None, None));
    assert!(!cantrip_api::Query::chars(&g, cart).power_above_base());
    effect(&mut g, cart, EffectKind::Pump(1, 1));
    assert_eq!(read(&g, cart), (None, None));
    effect(&mut g, cart, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])));
    assert_eq!(read(&g, cart), (Some(4), Some(3)));
}

/// CR 400.7, 608.2h: a creature that has left is as it last was, its base power as well as its power, though the
/// effects on it have gone. The new object in the zone it went to is its card again.
#[test]
fn a_creature_that_has_left_keeps_its_base_power() {
    let mut g = game();
    let bear = enter(&mut g, &BEAR);
    effect(&mut g, bear, EffectKind::SetPt(4, 4));
    effect(&mut g, bear, EffectKind::Pump(3, 3));
    cantrip_api::Game::exile(&mut cx(&mut g), bear);
    assert_eq!(read(&g, bear), (Some(7), Some(4)));
    assert!(cantrip_api::Query::chars(&g, bear).power_above_base());
    let exiled = g.state.obj(bear).became.expect("a new object in exile");
    assert_eq!(read(&g, exiled), (Some(2), Some(2)));
}

/// CR 603.10, 120.3: the row a `Damage` event keeps has the source's base power as it dealt the damage, though it has
/// changed or left since. And immediately after the event, what a trigger's matcher reads of a source still there is the
/// source itself.
#[test]
fn the_damage_keeps_the_base_power_as_it_was_dealt() {
    let mut g = game();
    let bear = enter(&mut g, &BEAR);
    effect(&mut g, bear, EffectKind::Pump(1, 0));
    ops::damage(&mut cx(&mut g), bear, Target::Player(THEM), 3, true);
    let dealt = |g: &Game| cantrip_api::Query::history(g).damage_dealt().next().expect("the damage").chars(g);
    assert_eq!((dealt(&g).power, dealt(&g).base_power), (Some(3), Some(2)));
    assert!(cantrip_api::Query::chars(&g, bear).power_above_base());

    // It becomes something else: the damage says what it was.
    effect(&mut g, bear, EffectKind::SetPt(9, 9));
    assert_eq!((dealt(&g).power, dealt(&g).base_power), (Some(3), Some(2)));
    assert_eq!(read(&g, bear), (Some(10), Some(9)));

    // It leaves: the damage still says what it was, and it what it last was.
    cantrip_api::Game::exile(&mut cx(&mut g), bear);
    assert_eq!((dealt(&g).power, dealt(&g).base_power), (Some(3), Some(2)));
    assert!(dealt(&g).power_above_base());
    assert_eq!(read(&g, bear), (Some(10), Some(9)));
}

/// A 1+*/2+* whose characteristic-defining ability makes it 5/6, and a 1/2 once it has lost that ability (`lost_pt`, as Tarmogoyf's
/// 0/1 is).
static DEFINED_ONE_PLUS: CardDef = CardDef {
    name: "Base Power Defined One Plus",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    statics: &[characteristic_defining(Layer::PtDefine, five_six, "This creature's power is 1 plus 4 and its toughness is 2 plus 4.").lost_pt(1, 2)],
    ..CardDef::BLANK
};

fn plus_power_per_other_with_base_one(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let others = q.state().battlefield().iter().filter(|&&id| id != source && q.chars(id).base_power == Some(1)).count() as i32;
    chars.power = chars.power.map(|v| v + others);
}

/// A 3/3 with the shape of Zinnia, Valley's Voice: "gets +X/+0, where X is the number of other creatures you control with base
/// power 1", a read of other objects from within layer 7c, as the entries of the layer are applied.
static COUNTS_ONES: CardDef = CardDef {
    name: "Base Power Counts Ones",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    statics: &[static_ability(Layer::PtModify, itself, plus_power_per_other_with_base_one, "This creature gets +X/+0.")],
    ..CardDef::BLANK
};

fn game_with(defs: &[&'static CardDef]) -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new(defs.iter().copied())));
    Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

/// CR 707.2, 707.9b, 707.9d: a copy's base power is what it copied, a copied characteristic-defining ability's among it; a
/// stated power is its copiable value, and the characteristic-defining ability it replaces is not copied. Whatever it was
/// before it copied does not count.
#[test]
fn a_copy_has_the_base_power_its_copiable_values_give() {
    let mut g = game_with(&[&WALL, &BEAR, &DEFINED]);
    let wall = enter(&mut g, &WALL);
    let defined = g.cards.id_of(&DEFINED);
    let bear = g.cards.id_of(&BEAR);
    cantrip_api::Game::set_copy(&mut cx(&mut g), wall, Copiable::of(bear));
    assert_eq!(read(&g, wall), (Some(2), Some(2)));
    cantrip_api::Game::set_copy(&mut cx(&mut g), wall, Copiable::of(defined));
    assert_eq!(read(&g, wall), (Some(5), Some(5)));
    effect(&mut g, wall, EffectKind::Pump(1, 0));
    assert_eq!(read(&g, wall), (Some(6), Some(5)));
    cantrip_api::Game::set_copy(&mut cx(&mut g), wall, Copiable::of(defined).sized(3, 4));
    assert_eq!(read(&g, wall), (Some(4), Some(3)));
}

/// CR 708.2a, 613.2b: a face-down creature is a 2/2 by its copiable values, whatever its card is, under what modifies it; face
/// up its card's own values are its base power.
#[test]
fn a_face_down_creature_has_its_face_down_base_power() {
    let mut g = game_with(&[&DEFINED]);
    let mut card = Obj::new(g.cards.id_of(&DEFINED), YOU, Zone::library(YOU));
    card.timestamp = g.state.timestamp();
    let card = g.state.push_obj(card);
    g.state.zone_push(Zone::library(YOU), card);
    let down = cantrip_api::Game::put_onto_battlefield_face_down(&mut cx(&mut g), card, YOU, FaceDown::Manifested, false).expect("nothing is asked").expect("it enters face down");
    assert_eq!(read(&g, down), (Some(2), Some(2)));
    cantrip_api::Game::add_counters_by(&mut cx(&mut g), YOU, down, CounterKind::PlusOne, 1);
    assert_eq!(read(&g, down), (Some(3), Some(2)));
    assert!(cantrip_api::Query::chars(&g, down).power_above_base());
    g.state.update_obj(down, |o| {
        o.face_down = false;
        o.marks.face_down_as = None;
    });
    assert_eq!(read(&g, down), (Some(6), Some(5)));
}

/// CR 604.3, 613.1f: a creature that lost its characteristic-defining ability in layer 6 has what its box prints as its base
/// power, 7a finding the ability gone, and what modifies it on top.
#[test]
fn a_lost_characteristic_defining_ability_leaves_the_printed_base_power() {
    let mut g = game_with(&[&DEFINED_ONE_PLUS]);
    let defined = enter(&mut g, &DEFINED_ONE_PLUS);
    assert_eq!(read(&g, defined), (Some(5), Some(5)));
    effect(&mut g, defined, EffectKind::LoseAllAbilities);
    assert_eq!(read(&g, defined), (Some(1), Some(1)));
    effect(&mut g, defined, EffectKind::Pump(2, 0));
    assert_eq!(read(&g, defined), (Some(3), Some(1)));
    assert!(cantrip_api::Query::chars(&g, defined).power_above_base());
}

/// CR 613.4b, 613.4c: a static ability of layer 7c that asks another creature's base power (Zinnia, Valley's Voice) finds what
/// 7a and 7b made it, as they made it for every object, and nothing of 7c or 7d: its own count stands whatever modifies the
/// others, and no entry of 7c depends on another for it (CR 613.8).
#[test]
fn a_read_within_layer_seven_c_finds_the_base_power() {
    let mut g = game_with(&[&COUNTS_ONES, &WALL, &BEAR, &ANTHEM, &SHRINK]);
    let counts = enter(&mut g, &COUNTS_ONES);
    assert_eq!(read(&g, counts), (Some(3), Some(3)));
    let wall = enter(&mut g, &WALL);
    let bear = enter(&mut g, &BEAR);
    assert_eq!(read(&g, counts), (Some(4), Some(3)));
    // Modified, and switched: its base power is still 1.
    effect(&mut g, wall, EffectKind::Pump(2, 0));
    effect(&mut g, wall, EffectKind::SwitchPt);
    assert_eq!(read(&g, counts), (Some(4), Some(3)));
    enter(&mut g, &ANTHEM);
    assert_eq!(read(&g, counts), (Some(5), Some(3)));
    assert_eq!(read(&g, bear), (Some(3), Some(2)));
    // 7b makes every creature's base power 1: the Wall's, and the Bear's as well now.
    enter(&mut g, &SHRINK);
    assert_eq!(read(&g, counts), (Some(4), Some(1)));
}
