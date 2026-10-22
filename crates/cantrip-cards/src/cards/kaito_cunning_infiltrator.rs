use cantrip_api::*;
use cantrip_mechanics::triggers::creature_you_control_deals_combat_damage_to_player;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// The 2/1 blue Ninja creature token Kaito and his emblem make.
pub static NINJA: CardDef = CardDef {
    name: "Ninja Token",
    token_name: Some(&[Subtype::Ninja]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Ninja]),
    power: Some(2),
    toughness: Some(1),
    ..CardDef::BLANK
};

/// "Create a 2/1 blue Ninja creature token."
fn create_ninja(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &NINJA, 1)?;
    Ok(())
}

/// "Up to one target creature you control can't be blocked this turn (CR 509.1b, 611.2). Draw a card, then discard a card."
/// A target that became illegal stops the whole ability (CR 608.2b): nothing is drawn or discarded. With no target chosen
/// both still happen.
fn evade_and_loot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(&[Keyword::CantBeBlocked])), Duration::EndOfTurn);
    }
    effects::loot(g, item)
}

fn emblem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_emblem(g, item.controller, &EMBLEM);
    Ok(())
}

// --- The emblem ---------------------------------------------------------------------------------------------------

/// "Whenever a player casts a spell" (CR 601.2i, 603.2): any player's, the emblem's owner included. A spell cast, not a copy.
fn a_player_casts_a_spell(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    if matches!(event, Event::SpellCast { .. }) { Fired::PLAIN } else { None }
}

pub static EMBLEM: CardDef = CardDef {
    name: "Kaito, Cunning Infiltrator emblem",
    has_cost: false,
    abilities: &[triggered(
        when(EventMask::SPELL_CAST, a_player_casts_a_spell),
        create_ninja,
        "Whenever a player casts a spell, you create a 2/1 blue Ninja creature token.",
    )],
    text: "Whenever a player casts a spell, you create a 2/1 blue Ninja creature token.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        // "A creature you control deals combat damage to a player" (CR 510.2, 603.2): once for each creature.
        triggered(
            when_asked(EventMask::DAMAGE, creature_you_control_deals_combat_damage_to_player),
            effects::put_loyalty_counter_on_this,
            "Whenever a creature you control deals combat damage to a player, put a loyalty counter on Kaito.",
        ),
        ability(loyalty(1), evade_and_loot, "+1: Up to one target creature you control can't be blocked this turn. Draw a card, then discard a card.")
            .targets(&[targets::CREATURE_YOU_CONTROL.between(0, 1)]),
        ability(loyalty(-2), create_ninja, "\u{2212}2: Create a 2/1 blue Ninja creature token."),
        ability(loyalty(-9), emblem, "\u{2212}9: You get an emblem with \"Whenever a player casts a spell, you create a 2/1 blue Ninja creature token.\""),
    ],
    ..oracle::KAITO_CUNNING_INFILTRATOR
};

pub static EXTRA: &[&CardDef] = &[&NINJA, &EMBLEM];
