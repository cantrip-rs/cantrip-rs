use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}{W}"), effects::this_gets::<1, 0>, "{1}{W}: This creature gets +1/+0 until end of turn."),
        ability(cost("{W}{W}"), effects::this_gets::<0, 2>, "{W}{W}: This creature gets +0/+2 until end of turn."),
    ],
    ..oracle::KJELDORAN_KNIGHT
};
