use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Untapping another permanent uses the shared target rule (CR 115.1); surveil is the shared keyword action (CR 701.25a).
pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, effects::untap_targets, "{T}: Untap another target permanent.").targets(&[targets::OTHER_PERMANENT]),
        ability(cost("{3}"), |g, item| effects::surveil(g, item, 1), "{3}: Surveil 1."),
    ],
    ..oracle::DVANA_TENDI_OPTIMISTIC_ENSIGN
};
