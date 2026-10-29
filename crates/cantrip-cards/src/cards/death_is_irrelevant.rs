use cantrip_api::*;
use cantrip_mechanics::{assimilate, statics, targets, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your end step, assimilate target creature card from a graveyard." Assimilate moves that card
/// onto the battlefield under this ability's controller with a +1/+1 counter, as a Borg artifact creature (CR 122.6,
/// 205.1b, 614.12).
fn assimilate_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    assimilate::assimilate(g, item, card)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        assimilate_target,
        "At the beginning of your end step, assimilate target creature card from a graveyard.",
    )
    .targets(&[targets::CREATURE_CARD_IN_A_GRAVEYARD])],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_artifact_creatures,
        statics::have_indestructible,
        "Artifact creatures you control have indestructible.",
    )],
    ..oracle::DEATH_IS_IRRELEVANT
};
