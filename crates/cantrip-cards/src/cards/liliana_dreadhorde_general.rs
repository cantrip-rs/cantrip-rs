use cantrip_api::*;
use cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_DIES;
use cantrip_mechanics::{each_opponent_keeps_one_of_each_type, each_player_sacrifices, effects, tokens};
use cantrip_oracle as oracle;

/// A 2/2 black Zombie creature token (CR 111.1, 111.4).
static ZOMBIE: CardDef = CardDef {
    name: "Zombie Token",
    token_name: Some(&[Subtype::Zombie]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Zombie]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

pub static EXTRA: &[&CardDef] = &[&ZOMBIE];

/// +1: "Create a 2/2 black Zombie creature token."
fn plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ZOMBIE, 1)?;
    Ok(())
}

/// −4: "Each player sacrifices two creatures of their choice" (CR 701.21a, 101.4): the active player chooses first, then each
/// other player in turn order knowing those choices, and all the chosen creatures are sacrificed at the same time; a player
/// with only one creature sacrifices it.
fn minus_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_player_sacrifices(g, item.object, 2, |q, id| q.chars(id).is(CardType::Creature))?;
    Ok(())
}

/// −9: "Each opponent chooses a permanent they control of each permanent type and sacrifices the rest" (CR 110.4, 701.21a):
/// the permanent types are artifact, battle, creature, enchantment, land and planeswalker; a permanent with several types
/// may be the choice for more than one.
fn minus_nine(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_opponent_keeps_one_of_each_type(
        g,
        item.controller,
        item.object,
        &[CardType::Artifact, CardType::Battle, CardType::Creature, CardType::Enchantment, CardType::Land, CardType::Planeswalker],
        |_| true,
    )
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        // "Whenever a creature you control dies, draw a card" (CR 603.10a): looked back on, so it triggers for each creature
        // that dies at the same time as Liliana herself.
        triggered(CREATURE_YOU_CONTROL_DIES, |g, item| effects::draw(g, item, 1), "Whenever a creature you control dies, draw a card."),
        ability(loyalty(1), plus_one, "+1: Create a 2/2 black Zombie creature token."),
        ability(loyalty(-4), minus_four, "\u{2212}4: Each player sacrifices two creatures of their choice."),
        ability(loyalty(-9), minus_nine, "\u{2212}9: Each opponent chooses a permanent they control of each permanent type and sacrifices the rest."),
    ],
    ..oracle::LILIANA_DREADHORDE_GENERAL
};
