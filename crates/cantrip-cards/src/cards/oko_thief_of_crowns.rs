use cantrip_api::*;
use cantrip_mechanics::{effects, exchange};
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

fn food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::FOOD, 1)?;
    Ok(())
}

/// "Target artifact or creature loses all abilities and becomes a green Elk creature with base power and toughness
/// 3/3": for good. The card types are replaced, the subtypes of the lost ones going with them (CR 205.1a), and its creature
/// types are replaced by Elk (the ruling: "It's just a green Elk"), so an artifact's types go and a creature's are Elk alone.
fn elk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::loses_all_abilities_and_becomes_a_creature_for_good(g, item, 0, Color::Green, Subtype::Elk, 3, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(2), food, "+2: Create a Food token."),
        ability(loyalty(1), elk, "+1: Target artifact or creature loses all abilities and becomes a green Elk creature with base power and toughness 3/3.")
            .targets(&[targets::ARTIFACT_OR_CREATURE]),
        ability(loyalty(-5), exchange::control_of_two_targets, "\u{2212}5: Exchange control of target artifact or creature you control and target creature an opponent controls with power 3 or less.")
            .targets(&[
                targets::ARTIFACT_OR_CREATURE_YOU_CONTROL,
                targets::CREATURE_AN_OPPONENT_CONTROLS_WITH_POWER_3_OR_LESS,
            ]),
    ],
    ..oracle::OKO_THIEF_OF_CROWNS
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
