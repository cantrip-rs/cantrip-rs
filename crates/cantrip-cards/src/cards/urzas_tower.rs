use cantrip_api::*;
use cantrip_mechanics::{lands::sub, mana};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add_by(mana::add_colorless_if_you_control_the_other_urzas_lands::<{ sub(Subtype::Mine) }, { sub(Subtype::PowerPlant) }, 3>),
        "{T}: Add {C}. If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C} instead.",
    )],
    ..oracle::URZAS_TOWER
};
