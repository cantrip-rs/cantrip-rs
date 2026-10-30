//! The layer pass (CR 613) on boards the rules and rulings settle: real cards where the engine has them, and test-only
//! ones where it has none (Rune of Flight, the CR 613.9 pair, the loop of KI-61). Each board is put on the battlefield
//! directly, in the timestamp order the case needs.

use std::sync::OnceLock;

use cantrip_engine::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::statics::have_flying;

fn creatures(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}

fn lose_flying(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.keywords = Keywords(c.keywords.0 & !Keywords::of(&[Keyword::Flying]).0);
}

/// "Creatures have flying."
static WINGS: CardDef = CardDef {
    name: "Probe Wings",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Ability, creatures, have_flying, "Creatures have flying.")],
    ..CardDef::BLANK
};

/// "Creatures lose flying."
static CLIP: CardDef = CardDef {
    name: "Probe Clip",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Ability, creatures, lose_flying, "Creatures lose flying.")],
    ..CardDef::BLANK
};

/// A creature that prints flying.
static BIRD: CardDef = CardDef {
    name: "Probe Bird",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..CardDef::BLANK
};

fn enchanted(view: &dyn LayerQuery, source: ObjectId, id: ObjectId, _: &Chars) -> bool {
    view.state().obj(source).attached_to == Some(id)
}

fn give_flying_to_equipped(view: &dyn LayerQuery, source: ObjectId, c: &mut Chars) {
    let given = AbilityRef::delayed(view.state().obj(source).card, 0);
    if !c.abilities.contains(&Some(given))
        && let Some(slot) = c.abilities.iter().position(|a| a.is_none())
    {
        c.abilities[slot] = Some(given);
    }
}

/// Rune of Flight's shape (CR 613.7a's example): "Enchanted Equipment has 'Equipped creature has flying.'"
static RUNE: CardDef = CardDef {
    name: "Probe Rune of Flight",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Ability, enchanted, give_flying_to_equipped, "Enchanted Equipment has \"Equipped creature has flying.\"")],
    delayed: &[granted_static_ability(Layer::Ability, attached_object, have_flying, "Equipped creature has flying.")],
    ..CardDef::BLANK
};

fn lands(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Land)
}

fn add_swamp(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.subtypes = c.subtypes.with(Subtype::Swamp);
}

fn swamps(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Land) && c.has_subtype(Subtype::Swamp)
}

fn plains(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.set_land_subtypes(SubtypeSet::of(&[Subtype::Plains]));
}

/// A Swamp land with "Each land is a Swamp in addition to its other land types."
static BOG: CardDef = CardDef {
    name: "Probe Bog",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    subtypes: SubtypeSet::of(&[Subtype::Swamp]),
    statics: &[static_ability(Layer::Type, lands, add_swamp, "Each land is a Swamp in addition to its other land types.")],
    ..CardDef::BLANK
};

/// "All Swamps are Plains." (CR 305.7)
static BLEACH: CardDef = CardDef {
    name: "Probe Bleach",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[sets_land_subtypes(swamps, plains, "All Swamps are Plains.")],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&WINGS, &CLIP, &BIRD, &RUNE, &BOG, &BLEACH])))
}

/// A game with the named cards where named, controlled and owned by the first seat, with the timestamps given.
fn board(cards: &[(&str, Zone, u32)]) -> (Game, Vec<ObjectId>) {
    let filler = table().find("Plains").unwrap();
    let mut game = Game::new(table(), &Setup { decks: vec![vec![filler; 20], vec![filler; 20]], seed: 1, shuffle: false, ..Setup::default() });
    let ids = cards
        .iter()
        .map(|&(name, zone, timestamp)| {
            let mut obj = Obj::new(table().find(name).unwrap_or_else(|| panic!("{name}")), PlayerId(0), zone);
            obj.timestamp = timestamp;
            let id = game.state.push_obj(obj);
            game.state.zone_push(zone, id);
            id
        })
        .collect();
    game.state.see_statics();
    (game, ids)
}

const BF: Zone = Zone::BATTLEFIELD;

/// CR 613.9's example: "has flying" and "loses flying" depend on neither, so the later wins, on a creature that prints flying
/// too.
#[test]
fn has_flying_and_loses_flying_apply_in_timestamp_order() {
    let (game, ids) = board(&[("Probe Wings", BF, 10), ("Probe Clip", BF, 20), ("Probe Bird", BF, 30)]);
    assert!(!game.chars(ids[2]).has(Keyword::Flying), "loses flying, the later");
    let (game, ids) = board(&[("Probe Clip", BF, 10), ("Probe Wings", BF, 20), ("Probe Bird", BF, 30)]);
    assert!(game.chars(ids[2]).has(Keyword::Flying), "has flying, the later");
}

/// Akroma's Memorial and a "loses all abilities" (Turn to Frog's): neither depends on the other, so the later wins.
#[test]
fn akromas_memorial_and_a_loss_of_all_abilities_apply_in_timestamp_order() {
    for (lost_at, flying) in [(20, true), (40, false)] {
        let (mut game, ids) = board(&[("Llanowar Elves", BF, 10), ("Akroma's Memorial", BF, 30)]);
        game.state.effect_push(ContinuousEffect {
            source: ids[0],
            controller: PlayerId(0),
            timestamp: lost_at,
            affected: ids[0],
            kind: EffectKind::LoseAllAbilities,
            duration: Duration::EndOfTurn,
        });
        assert_eq!(game.chars(ids[0]).has(Keyword::Flying), flying, "lost at {lost_at}");
    }
}

/// CR 613.7a's example: the ability Rune of Flight gives Colossus Hammer has the later of the two timestamps, and keeps its
/// place after the Hammer's own abilities when the Hammer is attached anew: the equipped creature has flying, and +10/+10.
#[test]
fn rune_of_flight_on_colossus_hammer_gives_flying_whichever_is_newer() {
    for rune_at in [40, 20] {
        let (mut game, ids) = board(&[("Llanowar Elves", BF, 10), ("Colossus Hammer", BF, 30), ("Probe Rune of Flight", BF, rune_at)]);
        game.state.update_obj(ids[1], |o| o.attached_to = Some(ids[0]));
        game.state.update_obj(ids[2], |o| o.attached_to = Some(ids[1]));
        let elves = game.chars(ids[0]);
        assert!(elves.has(Keyword::Flying), "Rune at {rune_at}: flying");
        assert_eq!((elves.power, elves.toughness), (Some(11), Some(11)));
    }
}

/// Humility and Opalescence (the rulings): Opalescence makes Humility a creature in layer 4; their layer-7b parts go on in
/// timestamp order. Humility the earlier: it is a 4/4. Opalescence the earlier: it is a 1/1. Every other creature is 1/1.
#[test]
fn humility_and_opalescence_in_both_orders() {
    let (game, ids) = board(&[("Humility", BF, 10), ("Opalescence", BF, 20), ("Llanowar Elves", BF, 30)]);
    let humility = game.chars(ids[0]);
    assert!(humility.is(CardType::Creature) && (humility.power, humility.toughness) == (Some(4), Some(4)));
    assert_eq!(game.chars(ids[2]).power, Some(1));
    assert!(!game.chars(ids[1]).is(CardType::Creature), "Opalescence animates only other enchantments");
    let (game, ids) = board(&[("Opalescence", BF, 10), ("Humility", BF, 20), ("Llanowar Elves", BF, 30)]);
    let humility = game.chars(ids[1]);
    assert!(humility.is(CardType::Creature) && (humility.power, humility.toughness) == (Some(1), Some(1)));
    assert_eq!(game.chars(ids[2]).power, Some(1));
}

/// Conspiracy and Arcane Adaptation reach creature cards in hand and library (CR 611.3a); neither changes what the other
/// applies to, so timestamp order: Conspiracy first, the card is a Goblin Saproling; Adaptation first, a Goblin alone.
#[test]
fn conspiracy_and_arcane_adaptation_in_hand_and_library_apply_in_timestamp_order() {
    for (conspiracy, adaptation, expected) in [(10, 20, vec![Subtype::Goblin, Subtype::Saproling]), (20, 10, vec![Subtype::Goblin])] {
        let (mut game, ids) = board(&[
            ("Conspiracy", BF, conspiracy),
            ("Arcane Adaptation", BF, adaptation),
            ("Llanowar Elves", Zone::hand(PlayerId(0)), 30),
            ("Llanowar Elves", Zone::library(PlayerId(0)), 31),
        ]);
        // Chosen as each entered, by its own text (CR 607.2d).
        for (id, value) in [(ids[0], Subtype::Goblin), (ids[1], Subtype::Saproling)] {
            let text = game.cards.linked_to(&game.state, id).text;
            let choices = <cantrip_state::Choices as cantrip_state::ChoiceRecord>::entered(text, cantrip_state::Entered { subtype: Some(value), ..Default::default() });
            game.state.update_obj(id, |o| o.marks.choices = choices);
        }
        for &card in &ids[2..] {
            let c = game.chars(card);
            let creature_types: Vec<Subtype> = c.subtypes.iter().filter(|t| t.is_creature_type()).collect();
            assert_eq!(creature_types, expected, "Conspiracy at {conspiracy}");
        }
    }
}

/// CR 702.26b: a phased-out permanent is treated as though it doesn't exist. A Woodland Changeling (a Saproling, no land)
/// on the battlefield makes Blood Moon depend on Life and Limb, and the two loop (timestamp order: Taiga a Saproling
/// creature Mountain); phased out, it is no object either effect could apply to, Blood Moon applies first, and Taiga is a
/// plain Mountain.
#[test]
fn a_phased_out_changeling_is_no_object_the_layers_judge() {
    let taiga = |changeling: Zone| {
        let (game, ids) = board(&[("Life and Limb", BF, 10), ("Taiga", BF, 20), ("Woodland Changeling", changeling, 30), ("Blood Moon", BF, 40)]);
        game.chars(ids[1])
    };
    let looped = taiga(BF);
    assert!(looped.is(CardType::Creature) && looped.has_subtype(Subtype::Saproling) && looped.has_subtype(Subtype::Mountain));
    let plain = taiga(Zone::PHASED_OUT);
    assert_eq!(plain.types, TypeSet::of(&[CardType::Land]));
    assert_eq!(plain.subtypes, SubtypeSet::of(&[Subtype::Mountain]));
}

/// Blood Moon and Urborg, Tomb of Yawgmoth, in either order: Blood Moon takes Urborg's ability (CR 305.7: its existence
/// depends on Blood Moon, CR 613.8a), so no land is a Swamp; Urborg is a Mountain.
#[test]
fn blood_moon_and_urborg_in_either_order_leave_no_swamp() {
    for (urborg, moon) in [(10, 20), (20, 10)] {
        let (game, ids) = board(&[("Urborg, Tomb of Yawgmoth", BF, urborg), ("Blood Moon", BF, moon), ("Plains", BF, 30)]);
        assert_eq!(game.chars(ids[0]).subtypes, SubtypeSet::of(&[Subtype::Mountain]), "Urborg at {urborg}");
        assert_eq!(game.chars(ids[2]).subtypes, SubtypeSet::of(&[Subtype::Plains]), "Urborg at {urborg}");
        assert!(game.lost_all_abilities(ids[0]), "Urborg lost its rules text");
    }
}

/// KI-61's loop: a Swamp land whose own ability makes each land a Swamp, and "All Swamps are Plains" (CR 305.7). Each changes
/// the other's existence or reach (CR 613.8a), so they apply in timestamp order (CR 613.8b). The Bog first: the Mountain
/// becomes a Swamp, then both are Plains. The Bleach first: the Bog is a Plains and loses its ability before it applies, and
/// the Mountain stays a Mountain.
#[test]
fn a_lands_own_ability_and_a_land_type_setter_in_a_loop_apply_in_timestamp_order() {
    let (game, ids) = board(&[("Probe Bog", BF, 10), ("Probe Bleach", BF, 20), ("Mountain", BF, 30)]);
    assert_eq!(game.chars(ids[2]).subtypes, SubtypeSet::of(&[Subtype::Plains]));
    assert_eq!(game.chars(ids[0]).subtypes, SubtypeSet::of(&[Subtype::Plains]));
    let (game, ids) = board(&[("Probe Bleach", BF, 10), ("Probe Bog", BF, 20), ("Mountain", BF, 30)]);
    assert_eq!(game.chars(ids[2]).subtypes, SubtypeSet::of(&[Subtype::Mountain]));
    assert_eq!(game.chars(ids[1]).subtypes, SubtypeSet::of(&[Subtype::Plains]));
    assert!(game.lost_all_abilities(ids[1]), "the Bog lost its rules text");
}

fn plus_its_power(view: &dyn LayerQuery, source: ObjectId, c: &mut Chars) {
    // Its power as the layer stands, read through the view (the contract of `apply`): not from `c`, which it only writes.
    let power = view.chars(source).power.unwrap_or(0);
    c.power = c.power.map(|p| p + power);
}

fn itself(_: &dyn LayerQuery, source: ObjectId, id: ObjectId, _: &Chars) -> bool {
    source == id
}

/// "This creature gets +X/+0, where X is its power."
static DOUBLER: CardDef = CardDef {
    name: "Probe Doubler",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    statics: &[static_ability(Layer::PtModify, itself, plus_its_power, "This creature gets +X/+0, where X is its power.")],
    ..CardDef::BLANK
};

fn other_small_creatures(view: &dyn LayerQuery, holder: ObjectId) -> i16 {
    let you = view.state().obj(holder).controller;
    view.state()
        .battlefield()
        .iter()
        .filter(|&&other| other != holder && view.state().obj(other).controller == you)
        .filter(|&&other| {
            let c = view.chars(other);
            c.is(CardType::Creature) && c.power.is_some_and(|p| p <= 2)
        })
        .count() as i16
}

fn plus_per_small_creature(view: &dyn LayerQuery, holder: ObjectId, c: &mut Chars) {
    let n = other_small_creatures(view, holder);
    c.power = c.power.map(|p| p + n);
}

/// A creature an effect gives "This creature gets +1/+0 for each other creature you control with power 2 or less."
static RALLIER: CardDef = CardDef {
    name: "Probe Rallier",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    delayed: &[granted_static(Layer::PtModify, plus_per_small_creature, "This creature gets +1/+0 for each other creature you control with power 2 or less.")],
    ..CardDef::BLANK
};

fn artifacts(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Artifact)
}

fn creature_too(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.types = c.types.with(CardType::Creature);
}

fn plus_per_small_creature_each(view: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &mut Chars) {
    let you = view.state().obj(source).controller;
    let n = view
        .state()
        .battlefield()
        .iter()
        .filter(|&&other| other != id && other != source && view.state().obj(other).controller == you)
        .filter(|&&other| {
            let o = view.chars(other);
            o.is(CardType::Creature) && !o.is(CardType::Artifact) && o.power.is_some_and(|p| p <= 2)
        })
        .count() as i16;
    c.power = c.power.map(|p| p + n);
}

/// "Each artifact is a creature in addition to its other types and gets +1/+0 for each nonartifact creature you control with
/// power 2 or less": one effect in layers 4 and 7c (CR 613.6), what its later part does decided by other objects.
static FOUNDRY: CardDef = CardDef {
    name: "Probe Foundry",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[
        static_ability(Layer::Type, artifacts, creature_too, "Each artifact is a creature in addition to its other types"),
        static_ability_for_each(Layer::PtModify, artifacts, plus_per_small_creature_each, "and gets +1/+0 for each nonartifact creature you control with power 2 or less.")
            .continues(),
    ],
    ..CardDef::BLANK
};

static TRINKET: CardDef =
    CardDef { name: "Probe Trinket", types: TypeSet::of(&[CardType::Artifact]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };

fn more_table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&BIRD, &DOUBLER, &RALLIER, &FOUNDRY, &TRINKET])))
}

fn board_of(table: &'static CardTable, cards: &[(&str, u32)]) -> (Game, Vec<ObjectId>) {
    let filler = table.find("Plains").unwrap();
    let mut game = Game::new(table, &Setup { decks: vec![vec![filler; 20], vec![filler; 20]], seed: 1, shuffle: false, ..Setup::default() });
    let ids = cards
        .iter()
        .map(|&(name, timestamp)| {
            let mut obj = Obj::new(table.find(name).unwrap_or_else(|| panic!("{name}")), PlayerId(0), BF);
            obj.timestamp = timestamp;
            let id = game.state.push_obj(obj);
            game.state.zone_push(BF, id);
            id
        })
        .collect();
    game.state.see_statics();
    (game, ids)
}

fn pump(game: &mut Game, id: ObjectId, timestamp: u32) {
    game.state.effect_push(ContinuousEffect {
        source: id,
        controller: PlayerId(0),
        timestamp,
        affected: id,
        kind: EffectKind::Pump(2, 2),
        duration: Duration::EndOfTurn,
    });
}

/// CR 613.8a, "what it does": "+X/+0 where X is its power", the older, depends on Glorious Anthem, which changes its power,
/// and waits for it: a 2/2 is 3/3 under the Anthem, then +3/+0 - 6/3. (In timestamp order it would be 5/3.)
#[test]
fn an_effect_that_reads_its_own_object_waits_for_what_changes_it() {
    let (game, ids) = board_of(more_table(), &[("Probe Doubler", 10), ("Glorious Anthem", 20)]);
    let c = game.chars(ids[0]);
    assert_eq!((c.power, c.toughness), (Some(6), Some(3)));
}

/// A granted static ability (CR 613.1f) whose effect another 7c effect decides: the pump makes the other creature too big to
/// count, so the older ability depends on it and waits (CR 613.8a): +0, not +1.
#[test]
fn a_granted_static_ability_depends_through_what_it_does() {
    let (mut game, ids) = board_of(more_table(), &[("Probe Rallier", 10), ("Probe Bird", 11)]);
    let rallier = game.state.obj(ids[0]).card;
    game.state.effect_push(ContinuousEffect {
        source: ids[0],
        controller: PlayerId(0),
        timestamp: 12,
        affected: ids[0],
        kind: EffectKind::AddAbility(AbilityRef::delayed(rallier, 0)),
        duration: Duration::EndOfTurn,
    });
    assert_eq!(game.chars(ids[0]).power, Some(2), "+1/+0 for the Bird");
    pump(&mut game, ids[1], 30);
    assert_eq!(game.chars(ids[0]).power, Some(1), "the pumped Bird no longer counts");
}

/// CR 613.6 with 613.8a: the 7c part of an effect begun in layer 4 applies to the set its first part did, and what it does
/// still depends on another 7c effect - the pump that makes the Bird too big to count - and waits for it.
#[test]
fn a_continued_part_depends_through_what_it_does() {
    let (mut game, ids) = board_of(more_table(), &[("Probe Foundry", 10), ("Probe Trinket", 11), ("Probe Bird", 12)]);
    assert_eq!(game.chars(ids[1]).power, Some(2), "+1/+0 for the Bird");
    pump(&mut game, ids[2], 30);
    assert_eq!(game.chars(ids[1]).power, Some(1), "the pumped Bird no longer counts");
}

/// The static abilities whose effect changes nothing on any object or raw state the check tries, each with why.
const INERT: &[(&str, &str)] = &[
    (
        "Academy Instructor",
        "its effect counts Officer tokens its controller controls, and the synthetic boards hold none; a scenario of the card shows the +1/+0 for itself as an Officer token",
    ),
    (
        "Desert Were-Worm",
        "its effect counts the Mountains its controller controls, and the synthetic boards hold none; a scenario of the card shows the +2/+0 for each",
    ),
    (
        "Grand Nagus Zek",
        "its effect counts Treasures its controller controls, and the synthetic boards hold none; a scenario of the card shows the +1/+1 for each",
    ),
    (
        "Agatha's Soul Cauldron",
        "its effect gives the abilities of the creature cards exiled with it (`Obj::exiled`), a link no synthetic state makes; it reads them through the view and writes abilities only",
    ),
    (
        "Locus of Enlightenment",
        "its effect gives the activated abilities of the cards used to craft it, a link no synthetic state makes; it writes abilities only",
    ),
    (
        "Tomik, Orzhov Lawmage",
        "the ability it grants is a rule of the declaration of attackers (`CardDef::attack_limits`, CR 508.1c) that the core asks of Tomik; the granted ability's effect changes no characteristic",
    ),
    (
        "Wretched Bonemass",
        "its effect gives the keywords of the cards used to craft it, read through the view (`LayerQuery::chars`), a link no synthetic state makes",
    ),
    (
        "Kinbinding",
        "its effect counts the creatures the event history says entered under its controller's control this turn, and a synthetic state has no history; a scenario of the card shows the +X/+X",
    ),
];

/// The contract of every static ability's effect (`StaticApply`), across the whole card table: applied to synthetic objects
/// that differ in one field, each effect does what one instruction does to any object - it takes no value from the
/// object it is given. The corpus checks the effects it reaches under `CANTRIP_VERIFY`; this checks every one.
#[test]
fn no_static_ability_takes_a_value_from_its_object() {
    let found = apply_contract_check(more_table());
    assert!(found.breaches.is_empty(), "{}", found.breaches.join("\n"));
    let untested: Vec<&str> =
        found.inert.iter().map(|s| s.as_str()).filter(|line| !INERT.iter().any(|(name, _)| line.starts_with(name))).collect();
    assert!(untested.is_empty(), "changed nothing anywhere it was tried, so untested:\n{}", untested.join("\n"));
}

fn plus_its_power_from_chars(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    // Against the contract: its power read from the object it is given.
    c.power = c.power.map(|p| p + p);
}

/// "+X/+0, where X is its power", its power read from the object it is given: the check finds it.
static BAD_DOUBLER: CardDef = CardDef {
    name: "Probe Bad Doubler",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    statics: &[static_ability(Layer::PtModify, itself, plus_its_power_from_chars, "This creature gets +X/+0, where X is its power.")],
    ..CardDef::BLANK
};

#[test]
fn a_static_ability_that_takes_a_value_from_its_object_is_found() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&BAD_DOUBLER])));
    let breaches = apply_contract_check(table).breaches;
    assert_eq!(breaches.len(), 1, "{}", breaches.join("\n"));
    assert!(breaches[0].starts_with("Probe Bad Doubler"));
}

fn creature_an_opponent_stores(q: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &Chars) -> bool {
    // Against the contract of a control-changing reach: the stored controllers, not `LayerQuery::controller`.
    c.is(CardType::Creature) && q.state().obj(id).controller != q.state().obj(source).controller
}

/// "You control each creature an opponent controls", control read from the state: the check finds it.
static BAD_USURPER: CardDef = CardDef {
    name: "Probe Bad Usurper",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[control_changing(creature_an_opponent_stores, "You control each creature an opponent controls.")],
    ..CardDef::BLANK
};

#[test]
fn a_control_changing_reach_that_reads_the_stored_controller_is_found() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&BAD_USURPER])));
    let breaches = apply_contract_check(table).breaches;
    assert_eq!(breaches.len(), 1, "{}", breaches.join("
"));
    assert!(breaches[0].starts_with("Probe Bad Usurper"));
}

fn creature_while_you_store_another(q: &dyn LayerQuery, source: ObjectId, id: ObjectId, c: &Chars) -> bool {
    // Against the contract: "as long as you control another permanent", that permanent's control read from the state.
    let you = q.controller(source);
    c.is(CardType::Creature)
        && q.state().zone(Zone::BATTLEFIELD).iter().any(|&o| o != source && o != id && q.state().obj(o).controller == you)
}

/// "As long as you control another permanent, you control each creature", a third permanent's control read from the
/// state: the check flips every permanent of its scene, so it finds it.
static BAD_KEEPER: CardDef = CardDef {
    name: "Probe Bad Keeper",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[control_changing(
        creature_while_you_store_another,
        "As long as you control another permanent, you control each creature.",
    )],
    ..CardDef::BLANK
};

#[test]
fn a_control_changing_reach_that_reads_a_third_permanents_stored_controller_is_found() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&BAD_KEEPER])));
    let breaches = apply_contract_check(table).breaches;
    assert_eq!(breaches.len(), 1, "{}", breaches.join("\n"));
    assert!(breaches[0].starts_with("Probe Bad Keeper"));
}
