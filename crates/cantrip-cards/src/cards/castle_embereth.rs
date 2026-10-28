use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::lands::{enters_tapped_unless_you_control, sub};
use cantrip_oracle as oracle;

/// "This land enters tapped unless you control a Mountain." "{T}: Add {R}."
pub static CARD: CardDef = CardDef {
    enters: Some(enters_tapped_unless_you_control::<{ sub(Subtype::Mountain) }>),
    abilities: &[
        mana_ability(TAP, add(Mana::R), "{T}: Add {R}."),
        ability(cost("{1}{R}{R}, {T}"), effects::creatures_you_control_get::<1, 0>, "{1}{R}{R}, {T}: Creatures you control get +1/+0 until end of turn."),
    ],
    ..oracle::CASTLE_EMBERETH
};
