use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Return target instant or sorcery card from your graveyard to your hand." (CR 400.7: a target that left the graveyard is
/// illegal, CR 608.2b.) A preparation card is a creature card in every zone (CR 722.4), so it is not a target.
fn return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.return_to_hand(&[card]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, return_it, "When this creature enters, return target instant or sorcery card from your graveyard to your hand.")
        .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD])],
    ..oracle::ZEALOUS_LORECASTER
};
