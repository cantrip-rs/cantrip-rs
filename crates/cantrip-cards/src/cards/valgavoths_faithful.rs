use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{B}").and(SACRIFICE_THIS),
        effects::return_first_target_to_battlefield,
        "{3}{B}, Sacrifice this creature: Return target creature card from your graveyard to the battlefield. Activate only as a sorcery.",
    )
    .sorcery_speed()
    .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::VALGAVOTHS_FAITHFUL
};
