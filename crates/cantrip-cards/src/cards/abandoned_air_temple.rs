use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::lands::enters_tapped_unless_you_control_basic_land),
    abilities: &[
        mana_ability(TAP, add(Mana::W), "{T}: Add {W}."),
        ability(cost("{3}{W}, {T}"), effects::plus_one_counter_on_each_creature_you_control, "{3}{W}, {T}: Put a +1/+1 counter on each creature you control."),
    ],
    ..oracle::ABANDONED_AIR_TEMPLE
};
