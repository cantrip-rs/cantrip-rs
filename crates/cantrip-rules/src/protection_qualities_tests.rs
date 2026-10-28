//! Protection from a creature type and from a mana value (CR 702.16a-f): what the creature that has it can't be - the target of,
//! equipped by, blocked by, dealt damage by - judged against a source of that type or of that mana value, over test cards. The
//! Equipment path (CR 702.16d) is here because no printed Equipment is a creature type that a printed protection names.

use crate::{Game, Setup};
use cantrip_api::*;

fn itself(_: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected
}

fn from_dragons(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.protection_subtypes = chars.protection_subtypes.with(Subtype::Dragon);
}

fn from_mana_value_three_or_more(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.protection_mana_value = Some(3);
}

const DRAGON_WARDED_STATICS: &[StaticDef] = &[static_ability(Layer::Ability, itself, from_dragons, "Protection from Dragons")];
const VALUE_WARDED_STATICS: &[StaticDef] = &[static_ability(Layer::Ability, itself, from_mana_value_three_or_more, "Protection from mana value 3 or greater")];

static DRAGON_WARDED: CardDef = CardDef {
    name: "Dragon Warded Test Creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    statics: DRAGON_WARDED_STATICS,
    ..CardDef::BLANK
};
static VALUE_WARDED: CardDef = CardDef {
    name: "Value Warded Test Creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    statics: VALUE_WARDED_STATICS,
    ..CardDef::BLANK
};
/// An Equipment that is a Dragon, as a text-changed or changeling one would be.
static DRAGON_EQUIPMENT: CardDef = CardDef {
    name: "Dragon Test Equipment",
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment, Subtype::Dragon]),
    ..CardDef::BLANK
};
static PLAIN_EQUIPMENT: CardDef = CardDef {
    name: "Plain Test Equipment",
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    ..CardDef::BLANK
};
/// Equipment with mana value 3, and one with 2.
static COSTLY_EQUIPMENT: CardDef = CardDef {
    name: "Costly Test Equipment",
    cost: ManaCost { generic: 3, ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    ..CardDef::BLANK
};
static CHEAP_EQUIPMENT: CardDef = CardDef {
    name: "Cheap Test Equipment",
    cost: ManaCost { generic: 2, ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable =
        Box::leak(Box::new(CardTable::new([&DRAGON_WARDED, &VALUE_WARDED, &DRAGON_EQUIPMENT, &PLAIN_EQUIPMENT, &COSTLY_EQUIPMENT, &CHEAP_EQUIPMENT])));
    Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

fn put(game: &mut Game, def: &'static CardDef, player: u8) -> ObjectId {
    let mut obj = Obj::new(game.cards.id_of(def), PlayerId(player), Zone::BATTLEFIELD);
    obj.timestamp = game.state.timestamp();
    let id = game.state.push_obj(obj);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    // As a move onto the battlefield does (`ops::note_statics`): the layers look for static sources once any has arrived.
    if !def.statics.is_empty() {
        game.state.see_statics();
    }
    id
}

#[test]
fn a_dragon_equipment_can_not_equip_a_creature_with_protection_from_dragons_but_any_other_can() {
    let mut game = game();
    let host = put(&mut game, &DRAGON_WARDED, 0);
    let dragon = put(&mut game, &DRAGON_EQUIPMENT, 1);
    let plain = put(&mut game, &PLAIN_EQUIPMENT, 1);
    let q: &dyn Query = &crate::Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    assert!(q.chars(host).protection_subtypes.has(Subtype::Dragon));
    assert!(!crate::attach::host_allows(q, dragon, host, true), "a Dragon can't equip it (CR 702.16d)");
    assert!(crate::attach::host_allows(q, plain, host, true), "a non-Dragon Equipment can");
}

#[test]
fn an_equipment_of_mana_value_three_can_not_equip_a_creature_with_protection_from_mana_value_three_or_greater() {
    let mut game = game();
    let host = put(&mut game, &VALUE_WARDED, 0);
    let costly = put(&mut game, &COSTLY_EQUIPMENT, 1);
    let cheap = put(&mut game, &CHEAP_EQUIPMENT, 1);
    let q: &dyn Query = &crate::Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    assert_eq!(q.chars(host).protection_mana_value, Some(3));
    assert!(!crate::attach::host_allows(q, costly, host, true), "mana value 3 is covered (CR 702.16d)");
    assert!(crate::attach::host_allows(q, cheap, host, true), "mana value 2 is not");
}

#[test]
fn damage_targeting_and_blocking_read_the_same_judgment_of_a_source_by_type_and_mana_value() {
    let mut game = game();
    let dragon_host = put(&mut game, &DRAGON_WARDED, 0);
    let value_host = put(&mut game, &VALUE_WARDED, 0);
    let dragon = put(&mut game, &DRAGON_EQUIPMENT, 1);
    let plain = put(&mut game, &PLAIN_EQUIPMENT, 1);
    let costly = put(&mut game, &COSTLY_EQUIPMENT, 1);
    let cheap = put(&mut game, &CHEAP_EQUIPMENT, 1);
    let cx = crate::Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    let q: &dyn Query = &cx;
    let judged = |host: ObjectId, source: ObjectId| q.chars(host).protected_from_by(&q.chars(source), || q.mana_value(source), PlayerId(1));
    assert!(judged(dragon_host, dragon) && !judged(dragon_host, plain), "a Dragon source is covered, another type is not");
    assert!(!judged(dragon_host, costly), "mana value is not what protection from Dragons reads");
    assert!(judged(value_host, costly) && !judged(value_host, cheap), "mana value 3 is covered, 2 is not");
    assert!(!judged(value_host, dragon), "a Dragon type is not what protection from a mana value reads");
}
