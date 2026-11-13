use cantrip_api::*;
use cantrip_mechanics::tokens::{self, PILOT_SADDLES_AND_CREWS};
use cantrip_mechanics::effects::target_gains_hexproof_and_indestructible_until_end_of_turn as protect;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(protect, "Target permanent you control gains hexproof and indestructible until end of turn.").targets(&[targets::PERMANENT_YOU_CONTROL]),
    mode(tokens::create_a_pilot, "Create a 1/1 colorless Pilot creature token with \"This token saddles Mounts and crews Vehicles as though its power were 2 greater.\""),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::DEFEND_THE_RIDER };

pub static EXTRA: &[&CardDef] = &[&PILOT_SADDLES_AND_CREWS];
