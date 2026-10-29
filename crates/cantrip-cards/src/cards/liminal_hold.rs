use cantrip_api::*;
use cantrip_mechanics::{effects, exile_target_until_leaves, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Exile up to one target nonland permanent an opponent controls until this enchantment leaves the battlefield. You gain 2
/// life." (CR 610.3): the life is gained whether or not there was a target, or it is still legal.
fn exile_until_it_leaves_and_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target_until_leaves(g, item, 0);
    effects::gain_life(g, item, 2)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        exile_until_it_leaves_and_gain_life,
        "When this enchantment enters, exile up to one target nonland permanent an opponent controls until this enchantment leaves the battlefield. You gain 2 life.",
    )
    .targets(&[targets::NONLAND_PERMANENT_AN_OPPONENT_CONTROLS.between(0, 1)])],
    ..oracle::LIMINAL_HOLD
};
