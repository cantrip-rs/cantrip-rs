use cantrip_api::*;
use cantrip_mechanics::outlast;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[outlast::ability("{1}{B}", "Outlast {1}{B} ({1}{B}, {T}: Put a +1/+1 counter on this creature. Outlast only as a sorcery.)")],
    ..oracle::DISOWNED_ANCESTOR
};
