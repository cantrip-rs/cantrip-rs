use cantrip_api::*;
use cantrip_mechanics::statics::{enchanted_creatures_you_control, plus_pt};
use cantrip_mechanics::{targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The Role under the controller, attached to the target; nothing when the target is no longer legal (CR 608.2b).
fn create_monster_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::MONSTER_ROLE, item.controller, creature)?;
    Ok(())
}

/// "When Syr Armont enters, create a Monster Role token attached to another target creature you control." The Role is created and
/// attached by the effect, not cast; with the target illegal on resolution nothing happens (CR 608.2b), and a Role that could not
/// legally enchant it (protection from enchantments: Azorius First-Wing) is not created (CR 303.4i). Syr Armont itself is no legal
/// target (CR 115.1). "Enchanted creatures you
/// control get +1/+1" (CR 613.4c, layer 7c): any creature of its controller's with an Aura on it, whoever controls the Aura,
/// the Role included.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        create_monster_role,
        "When this creature enters, create a Monster Role token attached to another target creature you control.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    statics: &[static_ability(Layer::PtModify, enchanted_creatures_you_control, plus_pt::<1, 1>, "Enchanted creatures you control get +1/+1.")],
    ..oracle::SYR_ARMONT_THE_REDEEMER
};

pub static EXTRA: &[&CardDef] = &[&tokens::MONSTER_ROLE];
