use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_n(Mana::C, 2), "{T}: Add {C}{C}. Activate only if you control five or more lands.")
        .condition(cantrip_mechanics::you_control_at_least_n_lands::<5>)],
    ..oracle::TEMPLE_OF_THE_FALSE_GOD
};
