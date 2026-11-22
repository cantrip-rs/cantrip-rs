use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{B}, {T}").and(SACRIFICE_THIS),
        effects::return_first_target_to_battlefield_with_counters::<{ CounterKind::Finality as usize }, 1>,
        "{B}, {T}, Sacrifice this creature: Return target creature card from your graveyard to the battlefield with a finality counter on it. Activate only as a sorcery.",
    )
    .sorcery_speed()
    .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::SOULCOIL_VIPER
};
