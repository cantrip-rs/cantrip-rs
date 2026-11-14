use cantrip_api::*;
use cantrip_mechanics::effects::this_gets_and_gains_lifelink;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EERIE,
        this_gets_and_gains_lifelink::<2, 0>,
        "Eerie — Whenever an enchantment you control enters and whenever you fully unlock a Room, this creature gets +2/+0 and gains lifelink until end of turn.",
    )],
    ..oracle::DASHING_BLOODSUCKER
};
