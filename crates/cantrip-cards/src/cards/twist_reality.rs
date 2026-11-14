use cantrip_api::*;
use cantrip_mechanics::{effects, manifest_dread, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::counter_first_target, "Counter target spell.").targets(&[targets::SPELL]),
    mode(manifest_dread::controller_manifests_dread, "Manifest dread."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::TWIST_REALITY };
