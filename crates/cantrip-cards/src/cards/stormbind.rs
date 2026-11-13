use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{2}, Discard a card at random: This enchantment deals 2 damage to any target." (CR 602.1).
pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}")
                    .and(Cost {
                        discard_at_random: 1,
                        ..Cost::NONE
                    }),
                effects::damage_first_target::<2>,
                "{2}, Discard a card at random: This enchantment deals 2 damage to any target.",
            )
            .targets(&[targets::ANY]),
    ],
    ..oracle::STORMBIND
};
