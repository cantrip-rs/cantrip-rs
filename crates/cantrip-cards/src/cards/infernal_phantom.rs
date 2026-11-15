use cantrip_api::*;
use cantrip_mechanics::triggers::{EERIE, THIS_DIES};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            EERIE,
            effects::this_gets::<2, 0>,
            "Eerie — Whenever an enchantment you control enters and whenever you fully unlock a Room, this creature gets +2/+0 until end of turn.",
        ),
        triggered(THIS_DIES, effects::this_deals_damage_equal_to_its_power_to_first_target, "When this creature dies, it deals damage equal to its power to any target.")
            .targets(&[targets::ANY]),
    ],
    ..oracle::INFERNAL_PHANTOM
};
