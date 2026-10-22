//! The tokens the rules define by name (CR 111.10), as the C# `PredefinedTokens` names them.

use cantrip_api::*;
use crate::statics::itself;

/// A 1/1 colorless Thopter artifact creature token with flying (CR 111).
/// Pia, Determined Rebuilder and Retrofitter Foundry create this exact token.
pub static THOPTER: CardDef = CardDef {
    name: "Thopter Token",
    token_name: Some(&[Subtype::Thopter]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Thopter]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..CardDef::BLANK
};

/// A 1/1 colorless Myr artifact creature token (CR 111). Memnarch, the Warden and Myr Battlesphere create this exact token.
pub static MYR: CardDef = CardDef {
    name: "Myr Token",
    token_name: Some(&[Subtype::Myr]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Myr]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// A Royal Role token (CR 111.10m, 303.7): "a colorless Aura Role enchantment token named Royal with enchant creature and
/// 'Enchanted creature gets +1/+1 and has ward {1}.'" Royal Treatment, Charmed Clothier, Gylwain, Casting Director,
/// Besotted Knight and Redtooth Genealogist make it, attached to a creature (`create_role`).
pub static ROYAL_ROLE: CardDef = CardDef {
    name: "Royal",
    has_cost: false,
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura, Subtype::Role]),
    enchant: Some(crate::targets::CREATURE),
    delayed: &[crate::triggers::ward::<1>("Ward {1}")],
    statics: &[
        static_ability(Layer::PtModify, crate::attach::attached_object, crate::statics::plus_pt::<1, 1>, "Enchanted creature gets +1/+1."),
        static_ability(Layer::Ability, crate::attach::attached_object, crate::statics::grant_delayed_ability, "Enchanted creature has ward {1}."),
    ],
    text: "Enchant creature\nEnchanted creature gets +1/+1 and has ward {1}.",
    ..CardDef::BLANK
};

/// A Monster Role token (CR 111.10k, 303.7): "a colorless Aura Role enchantment token named Monster with enchant creature
/// and 'Enchanted creature gets +1/+1 and has trample.'" Monstrous Rage makes it, attached to a creature
/// (`create_role`).
pub static MONSTER_ROLE: CardDef = CardDef {
    name: "Monster",
    has_cost: false,
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura, Subtype::Role]),
    enchant: Some(crate::targets::CREATURE),
    statics: &[
        static_ability(Layer::PtModify, crate::attach::attached_object, crate::statics::plus_pt::<1, 1>, "Enchanted creature gets +1/+1."),
        static_ability(Layer::Ability, crate::attach::attached_object, crate::statics::have_trample, "Enchanted creature has trample."),
    ],
    text: "Enchant creature
Enchanted creature gets +1/+1 and has trample.",
    ..CardDef::BLANK
};

/// "Create a [Role] token attached to [a creature]" (CR 111.10j-r, 303.7, 303.4): a Role token `role` under `controller`,
/// entering attached to `creature`, which the effect names and does not target; not created if the Role could not
/// enchant it (CR 303.4i). If `controller` already controls a Role on it, the state-based action puts the older one into
/// its owner's graveyard (CR 704.5z). The card file lists `role` in its `EXTRA`. The tokens created.
pub fn create_role(g: &mut dyn Game, role: &'static CardDef, controller: PlayerId, creature: ObjectId) -> Result<Vec<ObjectId>, Suspend> {
    let role = g.card_id(role);
    g.create_token_attached(role, controller, creature)
}

fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// A colorless Clue artifact token with "{2}, Sacrifice this token: Draw a card." (CR 111.10f).
pub static CLUE: CardDef = CardDef {
    name: "Clue Token",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Clue]),
    abilities: &[ability(cost("{2}").and(SACRIFICE_THIS), draw_a_card, "{2}, Sacrifice this token: Draw a card.")],
    text: "{2}, Sacrifice this token: Draw a card.",
    ..CardDef::BLANK
};

fn add_one_of_any_color(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, color.unwrap_or(Color::White).mana(), 1);
    Ok(())
}

/// A colorless Treasure artifact token with "{T}, Sacrifice this artifact: Add one mana of any color." (CR 111.10a).
pub static TREASURE: CardDef = CardDef {
    name: "Treasure Token",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Treasure]),
    abilities: &[mana_ability(TAP.and(SACRIFICE_THIS), add_any_color(1).by(add_one_of_any_color), "{T}, Sacrifice this artifact: Add one mana of any color.")],
    text: "{T}, Sacrifice this artifact: Add one mana of any color.",
    ..CardDef::BLANK
};

/// Creates a Treasure token (CR 111.10a) under `controller`.
pub fn create_treasure(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    let treasure = g.card_id(&TREASURE);
    g.create_tokens(treasure, controller, 1, false)?;
    Ok(())
}

/// The Treasures `player` controls, token or not (CR 205.3g: Treasure is an artifact type), through the layers. Smaug the
/// Magnificent counts them and Swashbuckler Extraordinaire sacrifices them.
pub fn treasures_controlled(q: &dyn Query, player: PlayerId) -> Vec<ObjectId> {
    q.permanents_of(player).into_iter().filter(|&id| q.chars(id).has_subtype(Subtype::Treasure)).collect()
}

/// Creates a tapped Treasure token (CR 111.10a) under `controller`. Generous Plunderer and Death to Our Enemies print it.
pub fn create_tapped_treasure(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    let treasure = g.card_id(&TREASURE);
    g.create_tokens(treasure, controller, 1, true)?;
    Ok(())
}

fn any_card(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {
    true
}

/// A colorless Blood artifact token with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card." (CR 111.10g).
pub static BLOOD: CardDef = CardDef {
    name: "Blood Token",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Blood]),
    abilities: &[ability(cost("{1}, {T}").and(SACRIFICE_THIS).and(discard(1, any_card)), draw_a_card, "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")],
    text: "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.",
    ..CardDef::BLANK
};

/// Investigate (CR 701.16a): create a Clue token. Used by Thraben Inspector and Tireless Tracker.
pub fn investigate(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    let clue = g.card_id(&CLUE);
    g.create_tokens(clue, controller, 1, false)?;
    Ok(())
}

fn put_plus_one_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.target(0) {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    Ok(())
}

/// A colorless Mutagen artifact token with "{1}, {T}, Sacrifice this token: Put a +1/+1 counter on target creature.
/// Activate only as a sorcery." (CR 111.10v). Michelangelo, Weirdness to 11 and Mutagen Man print it.
pub static MUTAGEN: CardDef = CardDef {
    name: "Mutagen Token",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Mutagen]),
    abilities: &[ability(cost("{1}, {T}").and(SACRIFICE_THIS), put_plus_one_counter, "{1}, {T}, Sacrifice this token: Put a +1/+1 counter on target creature. Activate only as a sorcery.")
        .sorcery_speed()
        .targets(&[crate::targets::CREATURE])],
    text: "{1}, {T}, Sacrifice this token: Put a +1/+1 counter on target creature. Activate only as a sorcery.",
    ..CardDef::BLANK
};

fn explore_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Target::Object(creature) = item.target(0)
    {
        crate::explore(g, creature)?;
    }
    Ok(())
}

/// A colorless 2/2 Robot artifact creature token. Melded Moxite and Pinnacle Starcage create it. It shares its name with
/// the 1/1 Robot (Ravenous Robots'), so the card table keeps the definitions apart.
pub static ROBOT_2_2: CardDef = CardDef {
    name: "Robot Token",
    token_name: Some(&[Subtype::Robot]),
    has_cost: false,
    colors: Some(ColorSet::NONE),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Robot]),
    power: Some(2),
    toughness: Some(2),
    shares_name: true,
    ..CardDef::BLANK
};

/// A colorless Map artifact token with "{1}, {T}, Sacrifice this token: Target creature you control explores.
/// Activate only as a sorcery." (CR 111.10s). Fanatical Offering and Seasoned Dungeoneer make it.
pub static MAP: CardDef = CardDef {
    name: "Map Token",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Map]),
    abilities: &[ability(cost("{1}, {T}").and(SACRIFICE_THIS), explore_target, "{1}, {T}, Sacrifice this token: Target creature you control explores. Activate only as a sorcery.")
        .sorcery_speed()
        .targets(&[crate::targets::CREATURE_YOU_CONTROL])],
    text: "{1}, {T}, Sacrifice this token: Target creature you control explores. Activate only as a sorcery.",
    ..CardDef::BLANK
};

/// Creates a Map token (CR 111.10s) under `controller`.
pub fn create_map(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    let map = g.card_id(&MAP);
    g.create_tokens(map, controller, 1, false)?;
    Ok(())
}

fn gain_three_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}

/// A colorless Food artifact token with "{2}, {T}, Sacrifice this token: You gain 3 life." (CR 111.10b). Generous Ent
/// and Oko, Thief of Crowns create it.
pub static FOOD: CardDef = CardDef {
    name: "Food Token",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Food]),
    abilities: &[ability(cost("{2}, {T}").and(SACRIFICE_THIS), gain_three_life, "{2}, {T}, Sacrifice this token: You gain 3 life.")],
    text: "{2}, {T}, Sacrifice this token: You gain 3 life.",
    ..CardDef::BLANK
};

/// Creates a Food token (CR 111.10b) under `controller`.
pub fn create_food(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    let food = g.card_id(&FOOD);
    g.create_tokens(food, controller, 1, false)?;
    Ok(())
}

/// A 1/1 white Monk creature token with prowess. Cori-Steel Cutter's flurry and Monastery Mentor make it.
pub static MONK: CardDef = CardDef {
    name: "Monk Token",
    token_name: Some(&[Subtype::Monk]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Monk]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[crate::triggers::PROWESS],
    text: "Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)",
    ..CardDef::BLANK
};

/// The 1/1 blue and red Otter creature token with prowess. Ral, Crackling Wit's +1 and Stormchaser's Talent make it.
pub static OTTER: CardDef = CardDef {
    name: "Otter Token",
    token_name: Some(&[Subtype::Otter]),
    has_cost: false,
    colors: Some(ColorSet(ColorSet::of(Color::Blue).0 | ColorSet::of(Color::Red).0)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Otter]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[crate::triggers::PROWESS],
    text: "Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)",
    ..CardDef::BLANK
};

/// The 0/0 black Phyrexian Germ creature token living weapon makes (CR 702.92a): Nettlecyst, Kaldra Compleat.
pub static GERM: CardDef = CardDef {
    name: "Phyrexian Germ Token",
    token_name: Some(&[Subtype::Phyrexian, Subtype::Germ]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Phyrexian, Subtype::Germ]),
    power: Some(0),
    toughness: Some(0),
    ..CardDef::BLANK
};

/// The colorless 3/3 Robot Villain artifact creature token named Doombot (CR 111.4): its name is "Doombot", it has no
/// mana cost or color, and its subtypes are Robot and Villain. Castle Doom and Doctor Doom make it.
pub static DOOMBOT: CardDef = CardDef {
    name: "Doombot",
    has_cost: false,
    colors: Some(ColorSet::NONE),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Robot, Subtype::Villain]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};

/// A colorless 3/3 Golem artifact creature token. Legion Extruder and Sandstorm Salvager make it.
pub static GOLEM: CardDef = CardDef {
    name: "Golem Token",
    token_name: Some(&[Subtype::Golem]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Golem]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};

/// The 1/1 colorless Gnome artifact creature token. Cosmium Kiln and The Golden-Gear Colossus make it.
pub static GNOME: CardDef = CardDef {
    name: "Gnome Token",
    token_name: Some(&[Subtype::Gnome]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Gnome]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// The 1/1 colorless Pilot creature token: "This token crews Vehicles as though its power were 2 greater." Shorikai,
/// Genesis Engine and Reckoner Bankbuster make it.
pub static PILOT: CardDef = CardDef {
    name: "Pilot Token",
    token_name: Some(&[Subtype::Pilot]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Pilot]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::CrewsAsThoughPowerTwoGreater]),
    text: "This token crews Vehicles as though its power were 2 greater.",
    ..CardDef::BLANK
};

/// The 1/1 colorless Pilot creature token: "This token saddles Mounts and crews Vehicles as though its power were 2
/// greater." Defend the Rider, the Roads lands (Country Roads, Foul Roads, Reef Roads, Rocky Roads, Wild Roads),
/// Roadside Assistance, Back on Track, Cloudspire Coordinator and Valor's Flagship make it.
pub static PILOT_SADDLES_AND_CREWS: CardDef = CardDef {
    name: "Pilot Token",
    token_name: Some(&[Subtype::Pilot]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Pilot]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::SaddlesAsThoughPowerTwoGreater, Keyword::CrewsAsThoughPowerTwoGreater]),
    text: "This token saddles Mounts and crews Vehicles as though its power were 2 greater.",
    ..CardDef::BLANK
};

/// The 0/0 colorless Construct artifact creature token with "This token gets +1/+1 for each artifact you control". Urza's Saga
/// and Urza, Lord High Artificer make it.
pub static CONSTRUCT: CardDef = CardDef {
    name: "Construct Token",
    token_name: Some(&[Subtype::Construct]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Construct]),
    power: Some(0),
    toughness: Some(0),
    statics: &[static_ability(Layer::PtModify, itself, crate::statics::plus_one_plus_one_per_artifact_you_control, "This token gets +1/+1 for each artifact you control.")],
    text: "This token gets +1/+1 for each artifact you control.",
    ..CardDef::BLANK
};

/// The 0/0 colorless Phyrexian artifact creature an Incubator transforms into (CR 111.10i).
pub static PHYREXIAN: CardDef = CardDef {
    name: "Phyrexian Token",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Phyrexian]),
    power: Some(0),
    toughness: Some(0),
    ..CardDef::BLANK
};

fn transform_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.transform_by(item.object, item)
}

/// The double-faced Incubator artifact token (CR 111.10i, 701.53b): "{2}: Transform this token." Its counters are
/// those it was created with (`incubate`). A user lists this and `PHYREXIAN` in its `EXTRA`.
pub static INCUBATOR: CardDef = CardDef {
    name: "Incubator Token",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Incubator]),
    abilities: &[ability(cost("{2}"), transform_it, "{2}: Transform this token.")],
    back: Some(&PHYREXIAN),
    text: "{2}: Transform this token.",
    ..CardDef::BLANK
};

/// Incubate N (CR 701.53a): create an Incubator token with N +1/+1 counters on it. Sunfall and Metamorphosis-style
/// incubators share it.
pub fn incubate(g: &mut dyn Game, controller: PlayerId, n: i16) -> Result<(), Suspend> {
    let incubator = g.card_id(&INCUBATOR);
    g.create_tokens_with_counters(incubator, controller, 1, false, &[(CounterKind::PlusOne, n)])?;
    Ok(())
}

/// "When this token dies, you gain 1 life."
fn gain_a_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// The 1/1 black and green Pest creature token with "When this token dies, you gain 1 life."
pub static PEST: CardDef = CardDef {
    name: "Pest Token",
    token_name: Some(&[Subtype::Pest]),
    has_cost: false,
    colors: Some(ColorSet(0b10100)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Pest]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(crate::triggers::THIS_DIES, gain_a_life, "When this token dies, you gain 1 life.")],
    text: "When this token dies, you gain 1 life.",
    ..CardDef::BLANK
};

/// "Whenever this token attacks, you gain 1 life."
fn gain_a_life_on_attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// The 1/1 black and green Pest creature token with "Whenever this token attacks, you gain 1 life." Moseo, Vein's New Dean makes
/// it; the Pest with "When this token dies, you gain 1 life" is `PEST`.
pub static PEST_THAT_GAINS_LIFE_ON_ATTACK: CardDef = CardDef {
    name: "Pest Token",
    token_name: Some(&[Subtype::Pest]),
    has_cost: false,
    colors: Some(ColorSet(0b10100)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Pest]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(crate::triggers::THIS_ATTACKS, gain_a_life_on_attack, "Whenever this token attacks, you gain 1 life.")],
    text: "Whenever this token attacks, you gain 1 life.",
    ..CardDef::BLANK
};

/// A colorless Gold artifact token with "Sacrifice this artifact: Add one mana of any color." (CR 111.10c).
pub static GOLD: CardDef = CardDef {
    name: "Gold Token",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Gold]),
    abilities: &[mana_ability(SACRIFICE_THIS, add_any_color(1).by(add_one_of_any_color), "Sacrifice this artifact: Add one mana of any color.")],
    text: "Sacrifice this artifact: Add one mana of any color.",
    ..CardDef::BLANK
};

/// A red and green Heartwood artifact token with "{T}: Add {R} or {G}." (CR 111.10x). Aerid Konstrari makes it.
pub static HEARTWOOD: CardDef = CardDef {
    name: "Heartwood Token",
    has_cost: false,
    colors: Some(ColorSet(0b11000)),
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Heartwood]),
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Red, Color::Green]), "{T}: Add {R} or {G}.")],
    text: "{T}: Add {R} or {G}.",
    ..CardDef::BLANK
};

/// Creates a Heartwood token under `controller`.
pub fn create_heartwood(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create(g, controller, &HEARTWOOD, 1)?;
    Ok(())
}

/// Creates `count` tokens of `def` under `controller` (CR 111.1), untapped: the card file lists `def` in its `EXTRA`. The
/// tokens are created together (CR 101.4). Every card that makes a Soldier, a Servo or a Rebel calls it, as do those that
/// make a token of their own.
pub fn create(g: &mut dyn Game, controller: PlayerId, def: &'static CardDef, count: u32) -> Result<Vec<ObjectId>, Suspend> {
    let def = g.card_id(def);
    g.create_tokens(def, controller, count, false)
}

fn fetch_basic_land_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::lands::search_basic_land_to_battlefield(g, item.controller, item.object, true)
}

/// A colorless Lander artifact token with "{2}, {T}, Sacrifice this token: Search your library for a basic land card, put
/// it onto the battlefield tapped, then shuffle." (CR 111.10u). Biotech Specialist makes it.
pub static LANDER: CardDef = CardDef {
    name: "Lander Token",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Lander]),
    abilities: &[ability(
        cost("{2}").and(TAP).and(SACRIFICE_THIS),
        fetch_basic_land_tapped,
        "{2}, {T}, Sacrifice this token: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
    )],
    text: "{2}, {T}, Sacrifice this token: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
    ..CardDef::BLANK
};

/// Creates a Lander token (CR 111.10u) under `controller`.
pub fn create_lander(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create(g, controller, &LANDER, 1)?;
    Ok(())
}

/// Creates a Blood token (CR 111.10g) under `controller`. Bloodtithe Harvester and Ivora, Insatiable Heir make it.
pub fn create_blood(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create(g, controller, &BLOOD, 1)?;
    Ok(())
}

/// Gives `player` the emblem `def` (CR 114.2): an object in the command zone that nothing moves. The planeswalker lists
/// `def` in its `EXTRA`. Wrenn and Six and Teferi, Hero of Dominaria make theirs so.
pub fn create_emblem(g: &mut dyn Game, player: PlayerId, def: &'static CardDef) -> ObjectId {
    let def = g.card_id(def);
    g.create_emblem(def, player)
}

/// The 1/1 white Ally creature token. Appa, Steadfast Guardian and Iroh, Tea Master make it.
pub static ALLY: CardDef = CardDef {
    name: "Ally Token",
    token_name: Some(&[Subtype::Ally]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Ally]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// The 1/1 red Goblin creature token. Frontline Rush and Den of the Bugbear make it.
pub static GOBLIN: CardDef = CardDef {
    name: "Goblin Token",
    token_name: Some(&[Subtype::Goblin]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Goblin]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// The 1/1 green and white Citizen creature token (CR 111.4). Grand Crescendo and Rakish Revelers make it.
pub static CITIZEN: CardDef = CardDef {
    name: "Citizen Token",
    token_name: Some(&[Subtype::Citizen]),
    has_cost: false,
    colors: Some(ColorSet(0b10001)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Citizen]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// The 1/1 white Human creature token (CR 111.4). Adeline, Resplendent Cathar and Stroke of Midnight make it.
pub static HUMAN: CardDef = CardDef {
    name: "Human Token",
    token_name: Some(&[Subtype::Human]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Human]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// The 1/1 white Soldier creature token. Hero of Bladehold, Elspeth, Storm Slayer and Torsten, Founder of Benalia make
/// it. (Third Path Iconoclast's is an artifact Soldier of the same name.)
pub static SOLDIER: CardDef = CardDef {
    name: "Soldier Token",
    token_name: Some(&[Subtype::Soldier]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Soldier]),
    power: Some(1),
    toughness: Some(1),
    shares_name: true,
    ..CardDef::BLANK
};

/// The 4/4 white Angel creature token with flying and vigilance. Resplendent Angel and Parhelion II make it.
pub static ANGEL: CardDef = CardDef {
    name: "Angel Token",
    token_name: Some(&[Subtype::Angel]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Angel]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Vigilance]),
    ..CardDef::BLANK
};

/// The 2/1 green Spider creature token with reach. Spiders-Man, Heroic Horde and Peter Parker make it.
pub static SPIDER: CardDef = CardDef {
    name: "Spider Token",
    token_name: Some(&[Subtype::Spider]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Spider]),
    power: Some(2),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Reach]),
    ..CardDef::BLANK
};

/// The 1/1 green and white Human Citizen creature token. Silk, Web Weaver and News Helicopter make it.
pub static HUMAN_CITIZEN: CardDef = CardDef {
    name: "Human Citizen Token",
    token_name: Some(&[Subtype::Human, Subtype::Citizen]),
    has_cost: false,
    colors: Some(ColorSet(0b10001)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Human, Subtype::Citizen]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// The 1/1 white Human Soldier creature token. Cosmogrand Zenith makes it, and recruit (CR 701.70a: The Queen of Dale) does.
pub static HUMAN_SOLDIER: CardDef = CardDef {
    name: "Human Soldier Token",
    token_name: Some(&[Subtype::Human, Subtype::Soldier]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Human, Subtype::Soldier]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// The 2/2 red Rebel creature token. Glimmer Lens and Otharri, Suns' Glory make it.
pub static REBEL: CardDef = CardDef {
    name: "Rebel Token",
    token_name: Some(&[Subtype::Rebel]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Rebel]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// The 1/1 colorless Servo artifact creature token. Retrofitter Foundry and Saheeli, Sublime Artificer make it.
pub static SERVO: CardDef = CardDef {
    name: "Servo Token",
    token_name: Some(&[Subtype::Servo]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Servo]),
    power: Some(1),
    toughness: Some(1),
    shares_name: true,
    ..CardDef::BLANK
};

/// The 2/2 green Wolf creature token (CR 111.4). Chief Warg's Company and Head of the Hunt make it.
pub static WOLF: CardDef = CardDef {
    name: "Wolf Token",
    token_name: Some(&[Subtype::Wolf]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Wolf]),
    power: Some(2),
    toughness: Some(2),
    shares_name: true,
    ..CardDef::BLANK
};

/// A 1/1 blue Fish creature token (CR 702.174f). Into the Flood Maw's gift makes it, tapped.
pub static FISH: CardDef = CardDef {
    name: "Fish Token",
    token_name: Some(&[Subtype::Fish]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Fish]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// An 8/8 blue Octopus creature token (CR 702.174i). "Gift an Octopus" makes it.
pub static OCTOPUS: CardDef = CardDef {
    name: "Octopus Token",
    token_name: Some(&[Subtype::Octopus]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Octopus]),
    power: Some(8),
    toughness: Some(8),
    ..CardDef::BLANK
};

/// A 4/4 green Beast creature token with trample. Garruk, Curse Breaker and Garruk, Veiled Butcher make it.
pub static BEAST_4_4_TRAMPLE: CardDef = CardDef {
    name: "Beast Token",
    token_name: Some(&[Subtype::Beast]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Beast]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Trample]),
    ..CardDef::BLANK
};

/// A 1/1 white Rabbit creature token. Jacked Rabbit and Hop to It make it.
pub static RABBIT: CardDef = CardDef {
    name: "Rabbit Token",
    token_name: Some(&[Subtype::Rabbit]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Rabbit]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// The 2/2 colorless Wizard Soldier creature token named Cadet (CR 111.4). Ajani Unrelenting, Stingerquill Charm and
/// Ingris Stingerquill make it.
pub static CADET: CardDef = CardDef {
    name: "Cadet",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Wizard, Subtype::Soldier]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A 1/1 black Bat creature token with flying. Sanguine Evangelist and Aclazotz, Deepest Betrayal make it.
pub static BAT: CardDef = CardDef {
    name: "Bat Token",
    token_name: Some(&[Subtype::Bat]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Bat]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..CardDef::BLANK
};

/// The 1/1 black Rat creature token with "This token can't block." (CR 111.1, 509.1b). Song of Totentanz and Lord Skitter,
/// Sewer King make it; the card file lists it in its `EXTRA`.
pub static RAT_CANT_BLOCK: CardDef = CardDef {
    name: "Rat Token",
    token_name: Some(&[Subtype::Rat]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Rat]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::CantBlock]),
    text: "This token can't block.",
    ..CardDef::BLANK
};

/// "This token can't block or be blocked by non-Spirit creatures" (CR 509.1b): of any possible block it is in, the other
/// creature must be a Spirit, read through the token's words (CR 612).
fn spirit_blocking(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    if source != blocker && source != attacker {
        return true;
    }
    let other = if source == blocker { attacker } else { blocker };
    q.chars(other).has_subtype(q.words(source).subtype(Subtype::Spirit))
}

/// The 1/1 colorless Spirit creature token with "This token can't block or be blocked by non-Spirit creatures." Wan Shi
/// Tong, All-Knowing and Realm of Koh make it.
pub static SPIRIT: CardDef = CardDef {
    name: "Spirit Token",
    token_name: Some(&[Subtype::Spirit]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Spirit]),
    power: Some(1),
    toughness: Some(1),
    block_rules: &[spirit_blocking],
    shares_name: true,
    text: "This token can't block or be blocked by non-Spirit creatures.",
    ..CardDef::BLANK
};

/// The 1/1 white Spirit creature token with flying. Lingering Souls, Occult Epiphany, Staff of the Storyteller and Doomed
/// Traveler make it. It shares its name with the colorless [`SPIRIT`].
pub static WHITE_FLYING_SPIRIT: CardDef = CardDef {
    name: "Spirit Token",
    token_name: Some(&[Subtype::Spirit]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Spirit]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    shares_name: true,
    text: "Flying",
    ..CardDef::BLANK
};

/// "Create a Treasure token" as an effect (CR 111.10a): one Treasure under the controller of the resolving spell or ability. Bejeweled
/// Warg, Fable of the Mirror-Breaker's Reflection, Fountainport, Magda, Brazen Outlaw, Monument to Endurance, Sanar, Unfinished
/// Genius and Smaug the Magnificent print it.
pub fn create_a_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    create_treasure(g, item.controller)
}

/// The 0/0 black Goblin Army creature token amass Goblins makes when its controller has no Army (CR 701.47a). Down, Down to
/// Goblin Town and Azog, Moria's Ruin amass Goblins.
pub static GOBLIN_ARMY: CardDef = CardDef {
    name: "Goblin Army Token",
    token_name: Some(&[Subtype::Goblin, Subtype::Army]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Goblin, Subtype::Army]),
    power: Some(0),
    toughness: Some(0),
    ..CardDef::BLANK
};

/// A 5/5 red Dragon creature token with flying. Sarkhan, Fireblood and Kiora of Fire and Ashes make it, and Draconic
/// Visitor makes it instead of artifact tokens.
pub static DRAGON_5_5: CardDef = CardDef {
    name: "Dragon Token",
    token_name: Some(&[Subtype::Dragon]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Dragon]),
    power: Some(5),
    toughness: Some(5),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..CardDef::BLANK
};
