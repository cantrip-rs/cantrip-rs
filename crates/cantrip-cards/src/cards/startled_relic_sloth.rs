use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{exile_target, targets};
use cantrip_oracle as oracle;

/// "Exile up to one target card from a graveyard": nothing if the card is no longer there as the ability resolves (CR 608.2b).
fn exile_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target(g, item, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(BEGINNING_OF_YOUR_COMBAT, exile_a_card, "At the beginning of combat on your turn, exile up to one target card from a graveyard.")
        .targets(&[targets::CARD_IN_A_GRAVEYARD.optional()])],
    ..oracle::STARTLED_RELIC_SLOTH
};
