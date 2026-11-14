use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}").and(Cost { discard_at_random: 1, ..Cost::NONE }),
        effects::target_creature_gets::<2, 2>,
        "{3}, Discard a card at random: Target creature gets +2/+2 until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::CORAL_HELM
};
