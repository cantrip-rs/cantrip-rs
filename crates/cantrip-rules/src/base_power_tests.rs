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

/// Ornithopter's printed box, without any card implementation: a copy-exception CDA must replace this 0/2 in 7a.
static COPY_BODY: CardDef = CardDef {
    name: "CDA Copy Body",
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(0),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// Urabrask's Forge creates a token with stated 2/1 on its second trigger. Its stated box is inherited by a later copy.
static TOKEN_BODY: CardDef = CardDef {
    name: "CDA Copy Token Body",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    ..CardDef::BLANK
};

static ISLAND: CardDef = CardDef { name: "CDA Test Island", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static FOREST: CardDef = CardDef { name: "CDA Test Forest", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn differently_named_lands(q: &dyn LayerQuery, source: ObjectId, c: &mut Chars) {
    let mut names = std::collections::BTreeSet::new();
    for &id in q.state().battlefield() {
        if q.controller(id) == q.controller(source) {
            let land = q.chars(id);
            if land.is(CardType::Land) {
                names.insert(land.name);
            }
        }
    }
    c.power = Some(names.len() as i32);
    c.toughness = c.power;
}

/// Only the quoted ability of Colossal Clone Master's exception: a core fixture, not the printed card or its mechanic.
static COPY_CDA: CardDef = CardDef {
    name: "CDA Copy Exception",
    statics: &[characteristic_defining(Layer::PtDefine, differently_named_lands,
        "This creature's power and toughness are each equal to the number of differently named lands you control.")],
    ..CardDef::BLANK
};

fn copy_values(g: &mut Game, id: ObjectId, values: Copiable) {
    cantrip_api::Game::set_copy(&mut cx(g), id, values);
}

fn pt_and_base(g: &Game, id: ObjectId) -> (Option<i32>, Option<i32>, Option<i32>) {
    let c = cantrip_api::Query::chars(g, id);
    (c.power, c.toughness, c.base_power)
}

/// CR 604.3a, 707.9a: the ability acquired in the copying process is the copy's own CDA, over the copied printed box.
#[test]
fn a_copy_exception_cda_defines_pt_over_the_copied_printed_box() {
    let mut g = game_with(&[&COPY_BODY, &COPY_CDA, &ISLAND]);
    enter(&mut g, &ISLAND);
    let original = enter(&mut g, &COPY_BODY);
    let copy = enter(&mut g, &COPY_BODY);
    let values = cantrip_api::Query::copiable_values(&g, original).with_text(g.cards.id_of(&COPY_CDA));
    copy_values(&mut g, copy, values);
    assert_eq!(pt_and_base(&g, copy), (Some(1), Some(1), Some(1)));
    assert_eq!(pt_and_base(&g, original), (Some(0), Some(2), Some(0)));
    effect(&mut g, copy, EffectKind::Pump(2, 3));
    assert_eq!(pt_and_base(&g, copy), (Some(3), Some(4), Some(1)));
    // Ovinize's ability-loss part leaves the copied box, not the exception's undefined/zero box.
    effect(&mut g, copy, EffectKind::LoseAllAbilities);
    assert_eq!(pt_and_base(&g, copy), (Some(2), Some(5), Some(0)));
}

/// CR 707.9a, 707.9d: an inherited stated box excludes earlier CDAs only. Clone subsequently copies the newly acquired
/// CDA as well as that box; it evaluates for the new object's controller and current lands, not a saved source value.
#[test]
fn a_later_copy_cda_overrides_inherited_stated_pt_and_is_copiable() {
    let mut g = game_with(&[&TOKEN_BODY, &COPY_BODY, &COPY_CDA, &ISLAND, &FOREST]);
    enter(&mut g, &ISLAND);
    let token = enter(&mut g, &TOKEN_BODY);
    g.state.update_obj(token, |o| o.token = true);
    let values = Copiable::of(g.cards.id_of(&TOKEN_BODY)).sized(2, 1);
    copy_values(&mut g, token, values);
    let copy = enter(&mut g, &COPY_BODY);
    let values = cantrip_api::Query::copiable_values(&g, token).nonlegendary().with_text(g.cards.id_of(&COPY_CDA));
    copy_values(&mut g, copy, values);
    assert_eq!(pt_and_base(&g, copy), (Some(1), Some(1), Some(1)));
    let clone = enter(&mut g, &COPY_BODY);
    let values = cantrip_api::Query::copiable_values(&g, copy);
    assert_eq!(values.pt(), Some((2, 1)), "the CDA does not erase the underlying copiable box");
    copy_values(&mut g, clone, values);
    g.state.update_obj(clone, |o| o.controller = THEM);
    assert_eq!(pt_and_base(&g, clone), (Some(0), Some(0), Some(0)));
    let forest = enter(&mut g, &FOREST);
    g.state.update_obj(forest, |o| o.controller = THEM);
    assert_eq!(pt_and_base(&g, clone), (Some(1), Some(1), Some(1)));
    enter(&mut g, &ISLAND);
    assert_eq!(pt_and_base(&g, copy), (Some(1), Some(1), Some(1)), "same-named lands count once");
    enter(&mut g, &FOREST);
    assert_eq!(pt_and_base(&g, copy), (Some(2), Some(2), Some(2)));
    cantrip_api::Game::exile(&mut cx(&mut g), copy);
    assert_eq!(pt_and_base(&g, clone), (Some(1), Some(1), Some(1)), "the original copy is not the CDA's source");
    effect(&mut g, clone, EffectKind::LoseAllAbilities);
    assert_eq!(pt_and_base(&g, clone), (Some(2), Some(1), Some(2)));
}

/// Quicksilver Gargantuan copying a copy with an acquired CDA must exclude it, and Clone copying the Gargantuan must
/// not bring it back. A subsequent Colossal-style copy acquisition can add a fresh instance of the same text.
#[test]
fn a_stated_pt_copy_excludes_acquired_cdas_but_a_later_acquisition_is_distinct() {
    let mut g = game_with(&[&COPY_BODY, &COPY_CDA, &ISLAND, &FOREST]);
    enter(&mut g, &ISLAND);
    let copy = enter(&mut g, &COPY_BODY);
    let text = g.cards.id_of(&COPY_CDA);
    let values = Copiable::of(g.cards.id_of(&COPY_BODY)).with_text(text).sized(7, 7);
    copy_values(&mut g, copy, values);
    assert_eq!(pt_and_base(&g, copy), (Some(7), Some(7), Some(7)));
    let clone = enter(&mut g, &COPY_BODY);
    let values = cantrip_api::Query::copiable_values(&g, copy);
    copy_values(&mut g, clone, values);
    assert_eq!(pt_and_base(&g, clone), (Some(7), Some(7), Some(7)));
    copy_values(&mut g, clone, values.with_text(text));
    assert_eq!(pt_and_base(&g, clone), (Some(1), Some(1), Some(1)));
    enter(&mut g, &FOREST);
    assert_eq!(pt_and_base(&g, clone), (Some(2), Some(2), Some(2)));
    effect(&mut g, clone, EffectKind::LoseAllAbilities);
    assert_eq!(pt_and_base(&g, clone), (Some(7), Some(7), Some(7)));
}

/// CR 707.9a, 400.7: an acquired CDA and its box undo and redo with the row; leaving keeps their last-known result,
/// while the new object has its printed card again, without the copy's text or exclusions.
#[test]
fn acquired_cdas_undo_redo_and_leave_as_new_objects() {
    let mut g = game_with(&[&COPY_BODY, &COPY_CDA, &ISLAND]);
    enter(&mut g, &ISLAND);
    let copy = enter(&mut g, &COPY_BODY);
    let mark = g.state.mark();
    let values = Copiable::of(g.cards.id_of(&COPY_BODY)).sized(2, 1).with_text(g.cards.id_of(&COPY_CDA));
    copy_values(&mut g, copy, values);
    assert_eq!(pt_and_base(&g, copy), (Some(1), Some(1), Some(1)));
    let (undos, redos) = g.state.step_back(mark);
    assert!(g.state.obj(copy).copy.is_none());
    assert_eq!(pt_and_base(&g, copy), (Some(0), Some(2), Some(0)));
    g.state.step_forward(undos, redos);
    assert_eq!(cantrip_api::Query::copiable_values(&g, copy), values);
    assert_eq!(pt_and_base(&g, copy), (Some(1), Some(1), Some(1)));
    cantrip_api::Game::exile(&mut cx(&mut g), copy);
    assert_eq!(pt_and_base(&g, copy), (Some(1), Some(1), Some(1)));
    let new = g.state.obj(copy).became.expect("a new object in exile");
    assert!(g.state.obj(new).copy.is_none());
    assert_eq!(pt_and_base(&g, new), (Some(0), Some(2), Some(0)));
    assert_eq!(g.cards.texts(&g.state, g.state.obj(new)).collect::<Vec<_>>(), [g.cards.id_of(&COPY_BODY)]);
}

/// CR 708.8: a copied creature turned face down (Ixidron) keeps its copy effect beneath the face-down values. Asking
/// what it would be face up must see the acquired CDA again, without leaking it into the actual face-down creature.
#[test]
fn a_face_up_query_recovers_the_copy_exceptions_cda() {
    let mut g = game_with(&[&COPY_BODY, &COPY_CDA, &BEAR, &ISLAND]);
    enter(&mut g, &ISLAND);
    let copy = enter(&mut g, &COPY_BODY);
    let values = Copiable::of(g.cards.id_of(&COPY_BODY)).sized(2, 1).with_text(g.cards.id_of(&COPY_CDA));
    copy_values(&mut g, copy, values);
    let down = g.cards.id_of(&BEAR);
    g.state.update_obj(copy, |o| {
        o.face_down = true;
        o.marks.face_down_as = Some(down);
    });
    assert_eq!(pt_and_base(&g, copy), (Some(2), Some(2), Some(2)));
    let up = cantrip_api::Query::chars_if_face_up(&g, copy);
    assert_eq!((up.power, up.toughness, up.base_power), (Some(1), Some(1), Some(1)));
    assert_eq!(pt_and_base(&g, copy), (Some(2), Some(2), Some(2)));
}

/// Installs the existing core merge representation, as a resolved mutate spell would: the new component on top or under.
fn merge_component(g: &mut Game, id: ObjectId, def: &'static CardDef, on_top: bool) {
    let card = g.cards.id_of(def);
    let prior = g.state.obj(id).copy;
    let part = cantrip_state::MergedPart {
        card, face: card, token: false, commander_id: None, copy: None, on_top, timestamp: g.state.timestamp(), prior, melded: false,
    };
    let mut merged = g.state.merge_of(id).cloned().unwrap_or(cantrip_state::Merge {
        own_face: g.cards.current(g.state.obj(id)), own_token: g.state.obj(id).token, own_commander_id: None,
        own_face_down: None, parts: Vec::new(),
    });
    merged.parts.push(part);
    g.state.set_merge(id, merged);
}

/// A mutate creature placed above a Colossal-style copy keeps the acquired CDA below, with its component link. Clone
/// copies the merged CDA too. Quicksilver Gargantuan excludes it, and later mutation must not resurrect that excluded CDA.
#[test]
fn a_merged_components_exception_cda_and_its_exclusions_survive_copying_and_mutation() {
    let mut g = game_with(&[&COPY_BODY, &COPY_CDA, &BEAR, &ISLAND]);
    enter(&mut g, &ISLAND);
    let copy = enter(&mut g, &COPY_BODY);
    let text = g.cards.id_of(&COPY_CDA);
    let body = g.cards.id_of(&COPY_BODY);
    copy_values(&mut g, copy, Copiable::of(body).with_text(text));
    merge_component(&mut g, copy, &BEAR, true);
    assert_eq!(pt_and_base(&g, copy), (Some(1), Some(1), Some(1)));
    let texts: Vec<_> = g.cards.texts_linked(&g.state, copy).collect();
    assert_eq!(texts, [
        (g.cards.id_of(&BEAR), None),
        (body, Some(g.cards.component_link(&g.state, copy, body))),
        (text, Some(g.cards.component_link(&g.state, copy, body))),
    ]);
    let clone = enter(&mut g, &COPY_BODY);
    let values = cantrip_api::Query::copiable_values(&g, copy);
    copy_values(&mut g, clone, values);
    assert_eq!(pt_and_base(&g, clone), (Some(1), Some(1), Some(1)));
    assert_eq!(g.cards.link_of_text(&g.state, clone, text), Some(g.cards.component_link(&g.state, clone, body)),
        "a copy of a merge keeps the exception's component link without being physically merged");
    // The loss is not copiable; a fresh Clone still has the merged CDA.
    effect(&mut g, copy, EffectKind::LoseAllAbilities);
    assert_eq!(pt_and_base(&g, copy), (Some(2), Some(2), Some(2)));
    assert_eq!(pt_and_base(&g, clone), (Some(1), Some(1), Some(1)));
    copy_values(&mut g, clone, values.sized(7, 7));
    assert_eq!(pt_and_base(&g, clone), (Some(7), Some(7), Some(7)));
    merge_component(&mut g, clone, &BEAR, false);
    assert_eq!(pt_and_base(&g, clone), (Some(7), Some(7), Some(7)), "rebuilding the merged sources keeps prior exclusions");
    let values = cantrip_api::Query::copiable_values(&g, clone).with_text(text);
    copy_values(&mut g, clone, values);
    assert_eq!(pt_and_base(&g, clone), (Some(1), Some(1), Some(1)), "a later acquired CDA is not excluded");
}

fn unused_copy_ability(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// Only an activated ability's definition: the self-copy value updates below use the core API, not a printed card.
static COPY_ABILITY: CardDef = CardDef {
    name: "Repeated Copy Ability Fixture",
    abilities: &[ability(TAP, unused_copy_ability, "{T}: Copy ability fixture.")],
    ..CardDef::BLANK
};

/// CR 113.2c, 707.9a: Stage's repeated self-copying can pass 255 instances. Reading actual copiable values, not the
/// bounded displayed ability list, preserves every instance for the next self-copy and for another object copying it.
#[test]
fn repeated_self_copy_preserves_activated_ability_values_past_255() {
    let mut g = game_with(&[&ISLAND, &COPY_ABILITY]);
    let stage = enter(&mut g, &ISLAND);
    let ability = AbilityRef { card: g.cards.id_of(&COPY_ABILITY), index: 0 };
    assert!(matches!(g.cards.ability(ability), AbilityDef::Activated(_)));
    for n in 1..=512 {
        let values = cantrip_api::Query::copiable_values(&g, stage).with_ability(ability);
        copy_values(&mut g, stage, values);
        let read = cantrip_api::Query::copiable_values(&g, stage);
        assert_eq!(read.extra_instances().collect::<Vec<_>>(), vec![ability; n]);
    }
    let values = cantrip_api::Query::copiable_values(&g, stage);
    let clone = enter(&mut g, &ISLAND);
    copy_values(&mut g, clone, values);
    assert_eq!(cantrip_api::Query::copiable_values(&g, clone), values);
    assert_eq!(cantrip_api::Query::chars(&g, clone).abilities().filter(|&a| a == ability).count(), MAX_ABILITIES);
    effect(&mut g, clone, EffectKind::LoseAllAbilities);
    assert!(!cantrip_api::Query::chars(&g, clone).abilities().any(|a| a == ability));
    assert_eq!(cantrip_api::Query::copiable_values(&g, clone).extra_instances().collect::<Vec<_>>(), vec![ability; 512]);
}

/// CR 113.2c, 707.9a: both the linked and unlinked text walks retain every instance past 255. A subsequent copy and
/// merge keep the counts and the component link, and a replacing copy excludes every existing instance.
#[test]
fn copy_and_merged_text_walks_keep_more_than_255_instances() {
    let mut g = game_with(&[&COPY_BODY, &COPY_CDA, &BEAR, &ISLAND]);
    enter(&mut g, &ISLAND);
    let copy = enter(&mut g, &COPY_BODY);
    let text = g.cards.id_of(&COPY_CDA);
    let mut values = Copiable::of(g.cards.id_of(&COPY_BODY));
    for _ in 0..512 {
        values = values.with_text(text);
    }
    copy_values(&mut g, copy, values);
    assert_eq!(g.cards.texts(&g.state, g.state.obj(copy)).filter(|&c| c == text).count(), 512);
    assert_eq!(pt_and_base(&g, copy), (Some(1), Some(1), Some(1)));
    let clone = enter(&mut g, &COPY_BODY);
    let values = cantrip_api::Query::copiable_values(&g, copy);
    copy_values(&mut g, clone, values);
    merge_component(&mut g, clone, &BEAR, true);
    let link = g.cards.component_link(&g.state, clone, g.cards.id_of(&COPY_BODY));
    assert_eq!(g.cards.texts_linked(&g.state, clone).filter(|&(c, l)| c == text && l == Some(link)).count(), 512);
    assert_eq!(pt_and_base(&g, clone), (Some(1), Some(1), Some(1)));
    let replaced = cantrip_api::Query::copiable_values(&g, clone).sized(7, 7);
    copy_values(&mut g, clone, replaced);
    assert_eq!(pt_and_base(&g, clone), (Some(7), Some(7), Some(7)));
    assert_eq!(g.cards.texts_linked_with_exclusions(&g.state, clone)
        .filter(|&(c, _, e)| c == text && e.contains(cantrip_state::CdaExclusions::PT)).count(), 512);
}

fn graveyard_creatures(q: &dyn LayerQuery, source: ObjectId, c: &mut Chars) {
    let count = q.state().zone(Zone::graveyard(q.controller(source))).iter().filter(|&&id| q.chars(id).is(CardType::Creature)).count() as i32;
    c.power = Some(count);
    c.toughness = Some(count);
}

/// Boneyard Mycodrax's */* CDA and star-valued box, to distinguish a lower component's fallback from the topmost box.
static STAR_BODY: CardDef = CardDef {
    name: "CDA Star Box Fixture",
    types: TypeSet::of(&[CardType::Creature]),
    statics: &[characteristic_defining(Layer::PtDefine, graveyard_creatures,
        "This creature's power and toughness are each equal to the number of creature cards in your graveyard.").lost_pt(0, 0)],
    ..CardDef::BLANK
};

/// Mutating above a Boneyard Mycodrax retains its CDA, but losing that CDA leaves the top component's box. Clone
/// copying the merged creature uses that same box on ability loss, not the lower component's star-valued box.
#[test]
fn losing_a_merged_star_cda_leaves_the_topmost_copied_box() {
    let mut g = game_with(&[&STAR_BODY, &BEAR]);
    let star = enter(&mut g, &STAR_BODY);
    let dead = Obj::new(g.cards.id_of(&BEAR), YOU, Zone::graveyard(YOU));
    let dead = g.state.push_obj(dead);
    g.state.zone_push(Zone::graveyard(YOU), dead);
    merge_component(&mut g, star, &BEAR, true);
    assert_eq!(pt_and_base(&g, star), (Some(1), Some(1), Some(1)));
    let clone = enter(&mut g, &BEAR);
    let values = cantrip_api::Query::copiable_values(&g, star);
    copy_values(&mut g, clone, values);
    assert_eq!(pt_and_base(&g, clone), (Some(1), Some(1), Some(1)));
    effect(&mut g, star, EffectKind::LoseAllAbilities);
    effect(&mut g, clone, EffectKind::LoseAllAbilities);
    assert_eq!(pt_and_base(&g, star), (Some(2), Some(2), Some(2)));
    assert_eq!(pt_and_base(&g, clone), (Some(2), Some(2), Some(2)));
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


/// CR 613.4a-d: setting one axis never snapshots the other; later changes to a CDA remain visible there.
#[test]
fn setting_one_axis_keeps_the_other_axis_independent_and_obeys_layer_order() {
    let cards=Box::leak(Box::new(CardTable::new([&BEAR,&WALL,&DEFINED,&ANTHEM])));
    let mut g=Game::new(cards,&Setup {decks:vec![Vec::new(),Vec::new()],..Default::default()});
    let creature=enter(&mut g,&BEAR);let anthem=enter(&mut g,&ANTHEM);
    effect(&mut g,creature,EffectKind::SetPower(0));
    let c=g.chars(creature);assert_eq!((c.power,c.toughness,c.base_power),(Some(1),Some(3),Some(0)));
    effect(&mut g,creature,EffectKind::SetToughness(7));
    let c=g.chars(creature);assert_eq!((c.power,c.toughness,c.base_power),(Some(1),Some(8),Some(0)));
    // Copying different printed P/T changes the unset base axis, not the selected power.
    g.state.effects_retain(|e| !matches!(e.kind,EffectKind::SetToughness(_)));
    let wall=g.cards.id_of(&WALL);g.state.update_obj(creature,|o|o.copy=Some(Copiable::of(wall)));
    let c=g.chars(creature);assert_eq!((c.power,c.toughness),(Some(1),Some(4)));
    let defined=g.cards.id_of(&DEFINED);g.state.update_obj(creature,|o|o.copy=Some(Copiable::of(defined)));
    let c=g.chars(creature);assert_eq!((c.power,c.toughness),(Some(1),Some(7)));
    effect(&mut g,creature,EffectKind::SwitchPt);
    let c=g.chars(creature);assert_eq!((c.power,c.toughness),(Some(7),Some(1)));
    // Removing the anthem still changes the independent axis under the switch.
    g.state.zone_remove(Zone::BATTLEFIELD,anthem);
    let c=g.chars(creature);assert_eq!((c.power,c.toughness),(Some(6),Some(0)));
}

/// CR 613.4b, 613.7: the last setting of each axis wins, with full signed native P/T and journalled rollback.
#[test]
fn one_axis_settings_respect_timestamps_signed_values_counters_and_rollback() {
    let cards=Box::leak(Box::new(CardTable::new([&BEAR])));
    let mut g=Game::new(cards,&Setup {decks:vec![Vec::new(),Vec::new()],..Default::default()});
    let creature=enter(&mut g,&BEAR);
    effect(&mut g,creature,EffectKind::SetPt(5,7));
    effect(&mut g,creature,EffectKind::SetPower(-2));
    let c=g.chars(creature);assert_eq!((c.power,c.toughness,c.base_power),(Some(-2),Some(7),Some(-2)));
    effect(&mut g,creature,EffectKind::SetToughness(i32::MAX));
    let mark=g.state.mark();let before=crate::loops::position(&g.state);
    effect(&mut g,creature,EffectKind::SetPower(i32::MIN));
    let c=g.chars(creature);assert_eq!((c.power,c.toughness),(Some(i32::MIN),Some(i32::MAX)));
    assert_ne!(crate::loops::position(&g.state),before);
    g.state.rollback(mark);assert_eq!(crate::loops::position(&g.state),before);
    effect(&mut g,creature,EffectKind::SetPt(3,4));
    let c=g.chars(creature);assert_eq!((c.power,c.toughness),(Some(3),Some(4)));
    let mut cx=cx(&mut g);cx.doer=Some(YOU);cantrip_api::Game::add_counters(&mut cx,creature,CounterKind::PlusOne,2);
    let c=cx.chars(creature);assert_eq!((c.power,c.toughness,c.base_power),(Some(5),Some(6),Some(3)));
}
