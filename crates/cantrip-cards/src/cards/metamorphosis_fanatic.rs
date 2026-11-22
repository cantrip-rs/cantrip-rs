use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    miracle: Some(mana("{1}{B}")),
    // The lifelink counter is one it enters with (CR 614.1c, 122.6), and grants lifelink through the layers (CR 122.1b).
    abilities: &[triggered(
        THIS_ENTERS,
        effects::return_first_target_to_battlefield_with_counters::<{ CounterKind::Lifelink as usize }, 1>,
        "When this creature enters, return up to one target creature card from your graveyard to the battlefield with a lifelink counter on it.",
    )
    .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.optional()])],
    ..oracle::METAMORPHOSIS_FANATIC
};
