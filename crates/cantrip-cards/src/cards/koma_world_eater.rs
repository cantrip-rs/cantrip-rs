use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::{ward, DEALS_COMBAT_DAMAGE_TO_PLAYER};
use cantrip_oracle as oracle;

/// The 3/3 blue Serpent creature token named Koma's Coil (CR 111.4).
pub static KOMAS_COIL: CardDef = CardDef {
    name: "Koma's Coil",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Serpent]),
    colors: Some(ColorSet(2)),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};

/// "Create four 3/3 blue Serpent creature tokens named Koma's Coil" (CR 111.1): the four are made together (CR 101.4), under the
/// ability's controller.
fn create_four_komas_coils(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &KOMAS_COIL, 4)?;
    Ok(())
}

/// "This spell can't be countered" and trample come from the Oracle keywords (CR 101.2, 702.19); ward {4} (CR 702.21) is the shared
/// ward trigger.
pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<4>("Ward {4} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {4}.)"),
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER,
            create_four_komas_coils,
            "Whenever Koma deals combat damage to a player, create four 3/3 blue Serpent creature tokens named Koma's Coil.",
        ),
    ],
    ..oracle::KOMA_WORLD_EATER
};

pub static EXTRA: &[&CardDef] = &[&KOMAS_COIL];
