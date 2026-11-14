use cantrip_api::*;
use cantrip_mechanics::{
    effects, sacrifice_one_of, targets,
    triggers::{ENCHANTED_CREATURE_DEALT_DAMAGE, THIS_ENTERS},
};
use cantrip_oracle as oracle;

/// "Target player sacrifices a creature of their choice other than enchanted creature" (CR 701.21a, 303.4): the creature this Aura is
/// attached to as the trigger resolves, or was when the Aura left (CR 608.2h), is not among those they may choose, so a player whose only creature is the enchanted one
/// sacrifices nothing. The player may be the Aura's controller; a target that is no longer legal ends the ability (CR 608.2b).
fn target_player_sacrifices_a_creature_other_than_enchanted(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let enchanted = g.obj(item.object).attached_to;
    let creatures = g.permanents_of(player).into_iter().filter(|&id| Some(id) != enchanted && g.chars(id).is(CardType::Creature)).collect();
    sacrifice_one_of(g, player, item.object, 1, creatures)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        triggered(
            THIS_ENTERS,
            target_player_sacrifices_a_creature_other_than_enchanted,
            "When this Aura enters, target player sacrifices a creature of their choice other than enchanted creature.",
        )
        .targets(&[targets::PLAYER]),
        // Damage dealt to it by several sources at once is one batch (CR 603.2c): the one trigger.
        triggered(ENCHANTED_CREATURE_DEALT_DAMAGE, effects::destroy_subject, "When enchanted creature is dealt damage, destroy it.").sums_per_subject(),
    ],
    ..oracle::SPOROGENIC_INFECTION
};
