use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return target creature card from your graveyard to the battlefield. You gain 3 life" (CR 110.2a, 608.2c): the card comes back
/// under your control as the first sentence, and the life is gained as a sentence of its own, whether or not the card could
/// return (a Grafdigger's Cage keeps it in the graveyard), unless the target is no longer legal, when the spell does not resolve
/// at all (CR 608.2b).
fn return_then_gain_three_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_battlefield(g, item)?;
    g.end_sentence();
    effects::gain_three_life(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], return_then_gain_three_life),
    ..oracle::EMERGE_FROM_THE_COCOON
};
