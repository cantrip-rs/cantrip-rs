use cantrip_api::*;
use cantrip_mechanics::outlast;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[outlast::ability("{1}{W}", "Outlast {1}{W} ({1}{W}, {T}: Put a +1/+1 counter on this creature. Outlast only as a sorcery.)")],
    ..oracle::SALT_ROAD_PATROL
};
