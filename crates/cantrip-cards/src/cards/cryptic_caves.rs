use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        lands::sacrifice_to_draw("{1}, {T}, Sacrifice this land: Draw a card. Activate only if you control five or more lands.").condition(cantrip_mechanics::you_control_at_least_n_lands::<5>),
    ],
    ..oracle::CRYPTIC_CAVES
};
