use cantrip_api::*;
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_mechanics::{damage_target, tokens};
use cantrip_oracle as oracle;

/// The 3/1 colorless Wall artifact creature token with defender named Stone Boulder (CR 111.4): its name is "Stone Boulder",
/// it has no mana cost or color, and its subtypes are Wall (read through the creating text's words, CR 612.2a).
pub static STONE_BOULDER: CardDef = CardDef {
    name: "Stone Boulder",
    has_cost: false,
    colors: Some(ColorSet::NONE),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Wall]),
    power: Some(3),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Defender]),
    text: "Defender",
    ..CardDef::BLANK
};

pub static EXTRA: &[&CardDef] = &[&STONE_BOULDER];

/// "Whenever this creature enters or attacks, create a 3/1 colorless Wall artifact creature token with defender named Stone
/// Boulder." One token each time, for the Giant's controller.
fn create_stone_boulder(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &STONE_BOULDER, 1)?;
    Ok(())
}

/// "This creature deals 4 damage to any target." Nothing if the target is no longer legal (CR 608.2b).
fn four_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 4);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS_OR_ATTACKS, create_stone_boulder, "Whenever this creature enters or attacks, create a 3/1 colorless Wall artifact creature token with defender named Stone Boulder."),
        ability(cost("{2}{R}").and(sacrifice(1, filters::artifact)), four_damage, "{2}{R}, Sacrifice an artifact: This creature deals 4 damage to any target.")
            .targets(&[targets::ANY]),
    ],
    ..oracle::STONE_GIANT_OF_HIGH_PASS
};
