use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{R}{G}"),
            effects::this_gets::<1, -1>,
            "{R}{G}: This creature gets +1/-1 until end of turn.",
        ),
    ],
    ..oracle::SHAMBLING_STRIDER
};
