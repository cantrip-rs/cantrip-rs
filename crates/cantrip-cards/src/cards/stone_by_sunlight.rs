use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature becomes an artifact in addition to its other types and gains indestructible" (CR
/// 205.1b, 613.1d, 611.2a): one instruction, both for the rest of the turn. Nothing happens if it is no longer a legal target
/// (CR 608.2b).
fn becomes_an_indestructible_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        g.add_effect(item.object, creature, EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact])), Duration::EndOfTurn);
        effects::gain_indestructible_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::destroy_first_target, "Destroy target creature with power 4 or greater.").targets(&[targets::CREATURE_WITH_POWER_4_OR_GREATER]),
    mode(becomes_an_indestructible_artifact, "Until end of turn, target creature becomes an artifact in addition to its other types and gains indestructible.")
        .targets(&[targets::CREATURE]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::STONE_BY_SUNLIGHT };
