//! The tokens the rules define by name (CR 111.10), as the C# `PredefinedTokens` names them.

use cantrip_api::*;
use crate::statics::itself;

/// A 1/1 colorless Thopter artifact creature token with flying (CR 111).
/// Pia, Determined Rebuilder and Retrofitter Foundry create this exact token.
pub static THOPTER: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..token!(Thopter)
};

/// A 1/1 colorless Myr artifact creature token (CR 111). Memnarch, the Warden and Myr Battlesphere create this exact token.
pub static MYR: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Myr)
};

/// A 3/3 colorless Illusion artifact creature token (CR 111). Adversary by Algorithm and Hologram Stalker create this exact
/// token.
pub static ILLUSION_3_3_ARTIFACT: CardDef = CardDef {
    colors: Some(ColorSet::NONE),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..token!(Illusion)
};

/// A 1/1 colorless Borg artifact creature token (CR 111). Antagonist Incorporator, Assembly Specialist, Locutus of Borg,
/// Conquest Mission and Join the Collective create this exact token.
pub static BORG: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Borg)
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
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{2}").and(SACRIFICE_THIS), draw_a_card, "{2}, Sacrifice this token: Draw a card.")],
    text: "{2}, Sacrifice this token: Draw a card.",
    ..token!(Clue)
};

fn add_one_of_any_color(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, color.unwrap_or(Color::White).mana(), 1);
    Ok(())
}

/// A colorless Treasure artifact token with "{T}, Sacrifice this artifact: Add one mana of any color." (CR 111.10a).
pub static TREASURE: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP.and(SACRIFICE_THIS), add_any_color(1).by(add_one_of_any_color), "{T}, Sacrifice this artifact: Add one mana of any color.")],
    text: "{T}, Sacrifice this artifact: Add one mana of any color.",
    ..token!(Treasure)
};

/// Creates a Treasure token (CR 111.10a) under `controller`.
pub fn create_treasure(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create_by_rule(g, controller, &TREASURE, 1, false)?;
    Ok(())
}

/// The Treasures `player` controls, token or not (CR 205.3g: Treasure is an artifact type), through the layers. Smaug the
/// Magnificent counts them and Swashbuckler Extraordinaire sacrifices them.
pub fn treasures_controlled(q: &dyn Query, player: PlayerId) -> Vec<ObjectId> {
    q.permanents_of(player).into_iter().filter(|&id| q.chars(id).has_subtype(Subtype::Treasure)).collect()
}

/// Creates a tapped Treasure token (CR 111.10a) under `controller`. Generous Plunderer and Death to Our Enemies print it.
pub fn create_tapped_treasure(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create_by_rule(g, controller, &TREASURE, 1, true)?;
    Ok(())
}

/// A colorless Powerstone artifact token with "{T}: Add {C}. This mana can't be spent to cast a nonartifact spell."
/// (CR 111.10h, 106.6). Cityscape Leveler, Excavation Explosion, Fallaji Excavation and Thran Spider make it.
pub static POWERSTONE: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add_n(Mana::C, 1).only_for(ManaRestriction::NotNonartifactSpells), "{T}: Add {C}. This mana can't be spent to cast a nonartifact spell.")],
    text: "{T}: Add {C}. This mana can't be spent to cast a nonartifact spell.",
    ..token!(Powerstone)
};

/// Creates a tapped Powerstone token (CR 111.10h) under `controller`: "creates a tapped Powerstone token" as the Brothers'
/// War cards print it (the token itself is not defined as entering tapped).
pub fn create_tapped_powerstone(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create_by_rule(g, controller, &POWERSTONE, 1, true)?;
    Ok(())
}

/// A colorless Blood artifact token with "{1}, {T}, Discard a card, Sacrifice this token: Draw a card." (CR 111.10g).
pub static BLOOD: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{1}, {T}").and(SACRIFICE_THIS).and(discard_cards(1)), draw_a_card, "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.")],
    text: "{1}, {T}, Discard a card, Sacrifice this token: Draw a card.",
    ..token!(Blood)
};

/// Investigate (CR 701.16a): create a Clue token. Used by Thraben Inspector and Tireless Tracker.
pub fn investigate(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create_by_rule(g, controller, &CLUE, 1, false)?;
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
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{1}, {T}").and(SACRIFICE_THIS), put_plus_one_counter, "{1}, {T}, Sacrifice this token: Put a +1/+1 counter on target creature. Activate only as a sorcery.")
        .sorcery_speed()
        .targets(&[crate::targets::CREATURE])],
    text: "{1}, {T}, Sacrifice this token: Put a +1/+1 counter on target creature. Activate only as a sorcery.",
    ..token!(Mutagen)
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
/// the 1/1 Robot (Ravenous Robots'), a definition of its own.
pub static ROBOT_2_2: CardDef = CardDef {
    colors: Some(ColorSet::NONE),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Robot)
};

/// A colorless Map artifact token with "{1}, {T}, Sacrifice this token: Target creature you control explores.
/// Activate only as a sorcery." (CR 111.10s). Fanatical Offering and Seasoned Dungeoneer make it.
pub static MAP: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{1}, {T}").and(SACRIFICE_THIS), explore_target, "{1}, {T}, Sacrifice this token: Target creature you control explores. Activate only as a sorcery.")
        .sorcery_speed()
        .targets(&[crate::targets::CREATURE_YOU_CONTROL])],
    text: "{1}, {T}, Sacrifice this token: Target creature you control explores. Activate only as a sorcery.",
    ..token!(Map)
};

/// Creates a Map token (CR 111.10s) under `controller`.
pub fn create_map(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create_by_rule(g, controller, &MAP, 1, false)?;
    Ok(())
}

fn gain_three_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}

/// A colorless Food artifact token with "{2}, {T}, Sacrifice this token: You gain 3 life." (CR 111.10b). Generous Ent
/// and Oko, Thief of Crowns create it.
pub static FOOD: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{2}, {T}").and(SACRIFICE_THIS), gain_three_life, "{2}, {T}, Sacrifice this token: You gain 3 life.")],
    text: "{2}, {T}, Sacrifice this token: You gain 3 life.",
    ..token!(Food)
};

/// Creates a Food token (CR 111.10b) under `controller`.
pub fn create_food(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create_by_rule(g, controller, &FOOD, 1, false)?;
    Ok(())
}

/// A 2/2 white Samurai creature token with vigilance. The Wandering Emperor's -1 and Experimental Synthesizer's sacrifice
/// ability make it.
pub static SAMURAI: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Vigilance]),
    text: "Vigilance",
    ..token!(Samurai)
};

/// A 1/1 white Monk creature token with prowess. Cori-Steel Cutter's flurry and Monastery Mentor make it.
pub static MONK: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[crate::triggers::PROWESS],
    text: "Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)",
    ..token!(Monk)
};

/// The 1/1 blue and red Otter creature token with prowess. Ral, Crackling Wit's +1 and Stormchaser's Talent make it.
pub static OTTER: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::Blue).0 | ColorSet::of(Color::Red).0)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[crate::triggers::PROWESS],
    text: "Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)",
    ..token!(Otter)
};

/// The 0/0 black Phyrexian Germ creature token living weapon makes (CR 702.92a): Nettlecyst, Kaldra Compleat.
pub static GERM: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    ..token!(Phyrexian Germ)
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
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..token!(Golem)
};

/// The 1/1 colorless Gnome artifact creature token. Cosmium Kiln and The Golden-Gear Colossus make it.
pub static GNOME: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Gnome)
};

/// The 1/1 colorless Pilot creature token: "This token crews Vehicles as though its power were 2 greater." Shorikai,
/// Genesis Engine and Reckoner Bankbuster make it.
pub static PILOT: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::CrewsAsThoughPowerTwoGreater]),
    text: "This token crews Vehicles as though its power were 2 greater.",
    ..token!(Pilot)
};

/// The 1/1 colorless Pilot creature token: "This token saddles Mounts and crews Vehicles as though its power were 2
/// greater." Defend the Rider, the Roads lands (Country Roads, Foul Roads, Reef Roads, Rocky Roads, Wild Roads),
/// Roadside Assistance, Back on Track, Cloudspire Coordinator and Valor's Flagship make it.
pub static PILOT_SADDLES_AND_CREWS: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::SaddlesAsThoughPowerTwoGreater, Keyword::CrewsAsThoughPowerTwoGreater]),
    text: "This token saddles Mounts and crews Vehicles as though its power were 2 greater.",
    ..token!(Pilot)
};

/// The 0/0 colorless Construct artifact creature token with "This token gets +1/+1 for each artifact you control". Urza's Saga
/// and Urza, Lord High Artificer make it.
pub static CONSTRUCT: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    statics: &[static_ability(Layer::PtModify, itself, crate::statics::plus_one_plus_one_per_artifact_you_control, "This token gets +1/+1 for each artifact you control.")],
    text: "This token gets +1/+1 for each artifact you control.",
    ..token!(Construct)
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
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{2}"), transform_it, "{2}: Transform this token.")],
    back: Some(&PHYREXIAN),
    text: "{2}: Transform this token.",
    ..token!(Incubator)
};

/// Incubate N (CR 701.53a): create an Incubator token with N +1/+1 counters on it. Sunfall and Metamorphosis-style
/// incubators share it.
pub fn incubate(g: &mut dyn Game, controller: PlayerId, n: i16) -> Result<(), Suspend> {
    let incubator = g.card_id(&INCUBATOR);
    // The Incubator is the rule's (CR 701.53a, 111.10i), as are the words of incubate's reminder text (CR 207.2a).
    g.create_tokens_with_counters(incubator, controller, 1, false, &[(CounterKind::PlusOne, n)], Words::NONE)?;
    Ok(())
}

/// "When this token dies, you gain 1 life."
fn gain_a_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// The 1/1 black and green Pest creature token with "When this token dies, you gain 1 life."
pub static PEST: CardDef = CardDef {
    colors: Some(ColorSet(0b10100)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(crate::triggers::THIS_DIES, gain_a_life, "When this token dies, you gain 1 life.")],
    text: "When this token dies, you gain 1 life.",
    ..token!(Pest)
};

/// "Whenever this token attacks, you gain 1 life."
fn gain_a_life_on_attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// The 1/1 black and green Pest creature token with "Whenever this token attacks, you gain 1 life." Moseo, Vein's New Dean makes
/// it; the Pest with "When this token dies, you gain 1 life" is `PEST`.
pub static PEST_THAT_GAINS_LIFE_ON_ATTACK: CardDef = CardDef {
    colors: Some(ColorSet(0b10100)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(crate::triggers::THIS_ATTACKS, gain_a_life_on_attack, "Whenever this token attacks, you gain 1 life.")],
    text: "Whenever this token attacks, you gain 1 life.",
    ..token!(Pest)
};

/// A colorless Gold artifact token with "Sacrifice this artifact: Add one mana of any color." (CR 111.10c).
pub static GOLD: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(SACRIFICE_THIS, add_any_color(1).by(add_one_of_any_color), "Sacrifice this artifact: Add one mana of any color.")],
    text: "Sacrifice this artifact: Add one mana of any color.",
    ..token!(Gold)
};

/// A red and green Heartwood artifact token with "{T}: Add {R} or {G}." (CR 111.10x). Aerid Konstrari, Heartwood Crafter, Konstrari Improviser and Hungering Puppetbeast make it.
pub static HEARTWOOD: CardDef = CardDef {
    colors: Some(ColorSet(0b11000)),
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Red, Color::Green]), "{T}: Add {R} or {G}.")],
    text: "{T}: Add {R} or {G}.",
    ..token!(Heartwood)
};

/// Creates a Heartwood token under `controller`.
pub fn create_heartwood(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create_by_rule(g, controller, &HEARTWOOD, 1, false)?;
    Ok(())
}

/// Creates `count` tokens of `def` under `controller` (CR 111.1), untapped: the card file lists `def` in its `EXTRA`. The
/// tokens are created together (CR 101.4). Every card that makes a Soldier, a Servo or a Rebel calls it, as do those that
/// make a token of their own.
pub fn create(g: &mut dyn Game, controller: PlayerId, def: &'static CardDef, count: u32) -> Result<Vec<ObjectId>, Suspend> {
    let def = g.card_id(def);
    g.create_tokens(def, controller, count, false)
}

/// Creates `count` tokens of `def` under `controller`, a token whose characteristics a rule defines and no printed text:
/// a predefined token (CR 111.10: a Treasure, a Clue, a Heartwood), a keyword's own (job select's Hero, recruit's Human
/// Soldier, a gift's Fish), or one created by name, whose characteristics are the Oracle card's of that name (CR 111.11:
/// Ginger, Queen of Sweets's Gingerbrute, made of Gingerbrute's own definition). A text change of the text making
/// it changes nothing of it (CR 612.2): its color words and creature types are not that text's, and a keyword's
/// reminder text is no rules text (CR 207.2a). Created together (CR 101.4), `tapped` as given.
pub fn create_by_rule(g: &mut dyn Game, controller: PlayerId, def: &'static CardDef, count: u32, tapped: bool) -> Result<Vec<ObjectId>, Suspend> {
    let def = g.card_id(def);
    g.create_tokens_worded(&vec![(def, controller); count as usize], tapped, None, Words::NONE)
}

fn fetch_basic_land_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    crate::lands::search_basic_land_to_battlefield(g, item.controller, item.object, true)
}

/// A colorless Lander artifact token with "{2}, {T}, Sacrifice this token: Search your library for a basic land card, put
/// it onto the battlefield tapped, then shuffle." (CR 111.10u). Biotech Specialist makes it.
pub static LANDER: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(
        cost("{2}").and(TAP).and(SACRIFICE_THIS),
        fetch_basic_land_tapped,
        "{2}, {T}, Sacrifice this token: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
    )],
    text: "{2}, {T}, Sacrifice this token: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
    ..token!(Lander)
};

/// Creates a Lander token (CR 111.10u) under `controller`.
pub fn create_lander(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create_by_rule(g, controller, &LANDER, 1, false)?;
    Ok(())
}

/// Creates a Blood token (CR 111.10g) under `controller`. Bloodtithe Harvester and Ivora, Insatiable Heir make it.
pub fn create_blood(g: &mut dyn Game, controller: PlayerId) -> Result<(), Suspend> {
    create_by_rule(g, controller, &BLOOD, 1, false)?;
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
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Ally)
};

/// The 1/1 red Goblin creature token. Frontline Rush and Den of the Bugbear make it.
pub static GOBLIN: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Goblin)
};

/// The 1/1 red Phyrexian Goblin creature token. Charforger, Chimney Rabble and Gleeful Demolition make it.
pub static PHYREXIAN_GOBLIN: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Phyrexian Goblin)
};

/// The 2/2 black Zombie creature token (CR 111.1, 111.4). Liliana, Dreadhorde General and Cryptcaller Chariot make it.
pub static ZOMBIE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Zombie)
};

/// The 1/1 green and white Citizen creature token (CR 111.4). Grand Crescendo and Rakish Revelers make it.
pub static CITIZEN: CardDef = CardDef {
    // White (bit 0) and green (bit 4).
    colors: Some(ColorSet(0b10001)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Citizen)
};

/// The 3/2 white Hero creature token with vigilance. Pet Avengers and Borough Backup make it.
pub static HERO_3_2_WHITE_VIGILANCE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Vigilance]),
    text: "Vigilance",
    ..token!(Hero)
};

/// The 2/1 black Villain creature token with menace. Agents of HYDRA, Arnim Zola, Bio-Fanatic, HYDRA Troopers, Madame Masque
/// and Construct a Cosmic Cube make it.
pub static VILLAIN_2_1_BLACK_MENACE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Menace]),
    text: "Menace",
    ..token!(Villain)
};

/// The 0/4 colorless Wall creature token with defender. Doctor Spectrum, Super-Skrull, The Fantastic Four, Wall Off and
/// Invisible Woman, Sue Storm make it.
pub static WALL_0_4_DEFENDER: CardDef = CardDef {
    colors: Some(ColorSet::NONE),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Defender]),
    text: "Defender",
    ..token!(Wall)
};

/// The 1/1 white Soldier creature token. Hero of Bladehold, Elspeth, Storm Slayer and Torsten, Founder of Benalia make
/// it. (Third Path Iconoclast's is an artifact Soldier of the same name.)
pub static SOLDIER: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Soldier)
};

/// The 2/2 red Mutant creature token. Jennika, Bad Apple Big Sister, Mighty Mutanimals, Sally Pride, Slash, Turtle Blimp, Wooden
/// Cane, Old Hob and Splinter & Leo make it.
pub static MUTANT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Mutant)
};

/// The 1/1 black Ninja creature token. Dark Leo & Shredder, Endless Foot Assault, Foot Chopper, Foot Mystic and Uneasy Alliance make it.
pub static NINJA: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Ninja)
};

/// "Create a 2/2 red Mutant creature token" as an effect (CR 111.1): one `MUTANT` under the controller of the resolving
/// spell or ability. Jennika, Bad Apple Big Sister and Mighty Mutanimals print it.
pub fn create_a_mutant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    create(g, item.controller, &MUTANT, 1)?;
    Ok(())
}

/// The 4/4 white Angel creature token with flying and vigilance. Resplendent Angel and Parhelion II make it.
pub static ANGEL: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Vigilance]),
    ..token!(Angel)
};

/// The 2/1 green Spider creature token with reach. Spiders-Man, Heroic Horde and Peter Parker make it.
pub static SPIDER: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Reach]),
    ..token!(Spider)
};

/// The 1/2 green Spider creature token with reach. Arasta of the Endless Web, Brood Weaver and Twin-Silk Spider make it.
pub static SPIDER_1_2: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Reach]),
    ..token!(Spider)
};

/// The 1/1 green and white Human Citizen creature token. Silk, Web Weaver and News Helicopter make it.
pub static HUMAN_CITIZEN: CardDef = CardDef {
    colors: Some(ColorSet(0b10001)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Human Citizen)
};

/// The 1/1 white Human creature token. Adeline, Resplendent Cathar and Castle Ardenvale make it.
pub static HUMAN: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Human)
};

/// A 2/2 red Dwarf creature token (CR 111). Fíli the Pathfinder, The Lonely Mountain, An Unexpected Party and Dwarven
/// Shortsword create this exact token.
pub static DWARF: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Dwarf)
};

/// The colorless Equipment artifact token named Axe, "Equipped creature gets +1/+0" and equip {2}: its name is "Axe"
/// (CR 111.4), it has no mana cost or color, and its subtype is Equipment (CR 301.5). Dáin Ironfoot and Iron Hills
/// Blacksmith create this exact token.
pub static AXE: CardDef = CardDef {
    name: "Axe",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    abilities: &[crate::attach::equip(mana("{2}"), "Equip {2}")],
    statics: &[static_ability(Layer::PtModify, crate::attach::attached_object, crate::statics::plus_pt::<1, 0>, "Equipped creature gets +1/+0.")],
    text: "Equipped creature gets +1/+0.\nEquip {2}",
    ..CardDef::BLANK
};

/// The 1/1 white Human Soldier creature token. Cosmogrand Zenith makes it, and recruit (CR 701.70a: The Queen of Dale) does.
pub static HUMAN_SOLDIER: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Human Soldier)
};

/// The 2/2 red Rebel creature token. Glimmer Lens and Otharri, Suns' Glory make it.
pub static REBEL: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Rebel)
};

/// The 1/1 colorless Servo artifact creature token. Retrofitter Foundry and Saheeli, Sublime Artificer make it.
pub static SERVO: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Servo)
};

/// The 2/2 green Wolf creature token (CR 111.4). Chief Warg's Company and Head of the Hunt make it.
pub static WOLF: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Wolf)
};

/// The 1/1 green Squirrel creature token (CR 111.4). Comet, Stellar Pup and The Unbeatable Squirrel Girl make it.
pub static SQUIRREL: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Squirrel)
};

/// A 1/1 blue Fish creature token (CR 702.174f). Into the Flood Maw's gift makes it, tapped.
pub static FISH: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Fish)
};

/// An 8/8 blue Octopus creature token (CR 702.174i). "Gift an Octopus" makes it.
pub static OCTOPUS: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(8),
    toughness: Some(8),
    ..token!(Octopus)
};

/// A 3/3 green Beast creature token. Garruk Wildspeaker and Thragtusk make it.
pub static BEAST_3_3: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..token!(Beast)
};

/// A 4/4 green Beast creature token with trample. Garruk, Curse Breaker and Garruk, Veiled Butcher make it.
pub static BEAST_4_4_TRAMPLE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Trample]),
    ..token!(Beast)
};

/// A 1/1 white Rabbit creature token. Jacked Rabbit and Hop to It make it.
pub static RABBIT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Rabbit)
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
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..token!(Bat)
};

/// The 1/1 black Rat creature token with "This token can't block." (CR 111.1, 509.1b). Song of Totentanz and Lord Skitter,
/// Sewer King make it; the card file lists it in its `EXTRA`.
pub static RAT_CANT_BLOCK: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::CantBlock]),
    text: "This token can't block.",
    ..token!(Rat)
};

/// The 1/1 colorless Phyrexian Mite artifact creature token with toxic 1 and "This token can't block." (CR 111.1, 702.164a,
/// 509.1b). Mirrex, Mite Overseer, Skrelv's Hive, Infested Fleshcutter, Crawling Chorus, Stinging Hivemaster and
/// Indoctrination Attendant make it; the card file lists it in its `EXTRA`.
pub static PHYREXIAN_MITE: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::CantBlock]),
    toxic: 1,
    text: "Toxic 1 (Players dealt combat damage by this creature also get a poison counter.)\nThis token can't block.",
    ..token!(Phyrexian Mite)
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
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    block_rules: &[spirit_blocking],
    text: "This token can't block or be blocked by non-Spirit creatures.",
    ..token!(Spirit)
};

/// The 1/1 white Spirit creature token with flying. Lingering Souls, Occult Epiphany, Staff of the Storyteller and Doomed
/// Traveler make it. It shares its name with the colorless [`SPIRIT`].
pub static WHITE_FLYING_SPIRIT: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Spirit)
};

/// The 1/1 white and black Inkling creature token with flying. Emeritus of Truce, Eager Glyphmage, Informed Inkwright and
/// Harsh Annotation make it.
pub static INKLING: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::White).0 | ColorSet::of(Color::Black).0)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Inkling)
};

/// The 2/2 red and white Spirit creature token. Antiquities on the Loose and Group Project make it.
pub static SPIRIT_2_2_RED_WHITE: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::Red).0 | ColorSet::of(Color::White).0)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Spirit)
};

/// "Create a Treasure token" as an effect (CR 111.10a): one Treasure under the controller of the resolving spell or ability. Bejeweled
/// Warg, Fable of the Mirror-Breaker's Reflection, Fountainport, Magda, Brazen Outlaw, Monument to Endurance, Sanar, Unfinished
/// Genius and Smaug the Magnificent print it.
pub fn create_a_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    create_treasure(g, item.controller)
}

/// "Target player creates N Treasure tokens" (CR 111.10a): all at once, under the player the item targeted in `slot`, if that
/// target is still legal (CR 608.2b). Ashling's Command (two) and Prismari Command (one) print it.
pub fn target_player_creates_treasures(g: &mut dyn Game, item: &StackItem, slot: usize, n: u32) -> Result<(), Suspend> {
    if let Some(player) = crate::targets::player(g, item, slot) {
        create_by_rule(g, player, &TREASURE, n, false)?;
    }
    Ok(())
}

/// The 0/0 black Goblin Army creature token amass Goblins makes when its controller has no Army (CR 701.47a). Down, Down to
/// Goblin Town and Azog, Moria's Ruin amass Goblins.
pub static GOBLIN_ARMY: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    ..token!(Goblin Army)
};

/// A 5/5 red Dragon creature token with flying. Sarkhan, Fireblood and Kiora of Fire and Ashes make it, and Draconic
/// Visitor makes it instead of artifact tokens.
pub static DRAGON_5_5: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(5),
    toughness: Some(5),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Dragon)
};

/// The 2/2 green Bear creature token (CR 111.1, 111.4). Argoth, Sanctum of Nature, Ayula's Influence, Bearscape, Caller of the
/// Claw, Dancing from Dark to Dawn, Path to the World Tree and Words of Wilding make it.
pub static BEAR: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Bear)
};

/// The 1/1 green Elf creature token (CR 111.1, 111.4). Down in the Valley's Saga, Thranduil the Strategist and Thranduil,
/// Sindarin Liege make it.
pub static ELF: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Elf)
};

/// "Create a 1/1 green Elf creature token" as an ability's whole effect (CR 111.1, 701.7a): one, under its controller. Down in the
/// Valley's Saga ability, Thranduil the Strategist and Thranduil, Sindarin Liege print it.
pub fn create_an_elf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    create(g, item.controller, &ELF, 1)?;
    Ok(())
}
