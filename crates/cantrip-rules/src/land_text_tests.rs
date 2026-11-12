//! CR 305.7: a land whose subtype an effect sets loses the abilities of its rules text, over test cards: Conversion's "All
//! Mountains are Plains", keyed on the subtype it overwrites, takes the text of the Mountains it applies to, and leaves a
//! land that is no Mountain alone.

use crate::{Game, Setup};
use cantrip_api::*;

fn mountain(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Land) && so_far.subtypes.has(Subtype::Mountain)
}

fn plains(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.set_land_subtypes(SubtypeSet::NONE.with(Subtype::Plains));
}

/// "All Mountains are Plains."
static CONVERSION: CardDef = CardDef {
    name: "Land Text Conversion",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[sets_land_subtypes(mountain, plains, "All Mountains are Plains.")],
    ..CardDef::BLANK
};

fn creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

fn plus_one(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.power = c.power.map(|p| p + 1);
    c.toughness = c.toughness.map(|t| t + 1);
}

/// A Mountain with a static ability of its rules text: "Creatures get +1/+1."
static PEAK: CardDef = CardDef {
    name: "Land Text Peak",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    subtypes: SubtypeSet::of(&[Subtype::Mountain]),
    statics: &[static_ability(Layer::PtModify, creature, plus_one, "Creatures get +1/+1.")],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef {
    name: "Land Text Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// The same, a Forest.
static GROVE: CardDef = CardDef { name: "Land Text Grove", subtypes: SubtypeSet::of(&[Subtype::Forest]), ..PEAK };

#[test]
fn an_ability_keyed_on_the_subtype_it_overwrites_takes_the_text_of_the_lands_it_applied_to() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&CONVERSION, &PEAK, &GROVE, &BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let put = |game: &mut Game, def: &'static CardDef| {
        let id = game.state.push_obj(Obj::new(table.id_of(def), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        game.state.see_statics();
        id
    };
    let bear = put(&mut game, &BEAR);
    let peak = put(&mut game, &PEAK);
    let grove = put(&mut game, &GROVE);
    // Two lands that each give creatures +1/+1.
    assert_eq!(game.chars(bear).power, Some(4));
    put(&mut game, &CONVERSION);
    assert!(game.chars(peak).subtypes.has(Subtype::Plains) && !game.chars(peak).subtypes.has(Subtype::Mountain));
    assert!(game.chars(grove).subtypes.has(Subtype::Forest));
    // The Mountain made a Plains has lost its rules text, and the Forest has not: one +1/+1 is left.
    assert_eq!(game.chars(bear).power, Some(3));
}

fn nonbasic_land(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Land) && !so_far.supertypes.basic
}

fn mountains(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.set_land_subtypes(SubtypeSet::NONE.with(Subtype::Mountain));
}

fn land(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Land)
}

fn also_a_swamp(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.subtypes = chars.subtypes.with(Subtype::Swamp);
}

fn swamp(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Land) && so_far.subtypes.has(Subtype::Swamp)
}

/// Blood Moon's "Nonbasic lands are Mountains."
static MOON: CardDef = CardDef {
    name: "Land Text Moon",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[sets_land_subtypes(nonbasic_land, mountains, "Nonbasic lands are Mountains.")],
    ..CardDef::BLANK
};
/// Urborg's "Each land is a Swamp in addition to its other land types."
static TOMB: CardDef = CardDef {
    name: "Land Text Tomb",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    statics: &[static_ability(Layer::Type, land, also_a_swamp, "Each land is a Swamp in addition to its other land types.")],
    ..CardDef::BLANK
};
/// "All Swamps are Plains."
static DRAIN: CardDef = CardDef {
    name: "Land Text Drain",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[sets_land_subtypes(swamp, plains, "All Swamps are Plains.")],
    ..CardDef::BLANK
};
/// A basic land with a static ability of its rules text: "Creatures get +1/+1."
static BASIC_PEAK: CardDef = CardDef {
    name: "Land Text Basic Peak",
    supertypes: Supertypes { basic: true, legendary: false, snow: false, world: false },
    subtypes: SubtypeSet::NONE,
    ..PEAK
};

/// The Moon takes the Tomb's text, so no land is a Swamp (CR 613.8a: the Tomb's ability depends on the Moon), and "All
/// Swamps are Plains" applies to nothing: the basic land keeps its rules text, and the Bear its +1/+1. While the lands that
/// lose their text are found, the Tomb's own ability no longer makes them Swamps once it is found among them.
#[test]
fn a_land_whose_text_is_lost_makes_no_land_a_swamp_while_they_are_found() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&MOON, &TOMB, &DRAIN, &BASIC_PEAK, &BEAR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let put = |game: &mut Game, def: &'static CardDef| {
        let id = game.state.push_obj(Obj::new(table.id_of(def), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        game.state.see_statics();
        id
    };
    let bear = put(&mut game, &BEAR);
    let peak = put(&mut game, &BASIC_PEAK);
    let tomb = put(&mut game, &TOMB);
    put(&mut game, &MOON);
    put(&mut game, &DRAIN);
    assert!(game.chars(tomb).subtypes.has(Subtype::Mountain));
    assert!(!game.chars(peak).subtypes.has(Subtype::Swamp));
    assert_eq!(game.chars(bear).power, Some(3));
}
