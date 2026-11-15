use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return up to two target creature cards from your graveyard to your hand. Each opponent loses 2 life and you gain 2 life."
/// (CR 608.2b-c, 119.3): the cards still legal return together, then the drain follows whether or not any came back; with
/// targets chosen and every one illegal the spell does not resolve at all, which is the core's business.
fn return_then_drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_each_target_to_hand(g, item)?;
    effects::each_opponent_loses_life_you_gain_life::<2>(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(0, 2)], return_then_drain),
    ..oracle::HAZELS_NOCTURNE
};
