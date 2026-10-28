use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{2}{W}{G}, {T}"), effects::plus_one_counter_on_each_creature_you_control, "{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control."),
    ],
    ..oracle::GAVONY_TOWNSHIP
};
