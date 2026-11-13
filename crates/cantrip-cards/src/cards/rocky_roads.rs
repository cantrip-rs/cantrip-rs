use cantrip_api::*;
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "This land enters tapped unless you control a Mount or Vehicle" (CR 614.1d, 614.12), "{T}: Add {R}." (CR 605.1a) and
    // "{1}{R}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token ... Activate only as a sorcery."
    // (CR 602.5d, 111.1): the shared Roads land.
    enters: Some(lands::enters_tapped_unless_you_control_either::<{ lands::sub(Subtype::Mount) }, { lands::sub(Subtype::Vehicle) }>),
    abilities: &lands::roads_land(
        Mana::R,
        "{T}: Add {R}.",
        cost("{1}{R}"),
        "{1}{R}, {T}, Sacrifice this land: Create a 1/1 colorless Pilot creature token with \"This token saddles Mounts and crews Vehicles as though its power were 2 greater.\" Activate only as a sorcery.",
    ),
    ..oracle::ROCKY_ROADS
};

pub static EXTRA: &[&CardDef] = &[&tokens::PILOT_SADDLES_AND_CREWS];
