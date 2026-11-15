use cantrip_api::*;
use cantrip_mechanics::{effects, manifest_dread::controller_manifests_dread, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::exile_first_target, "Exile target artifact.").targets(&[targets::ARTIFACT]),
    mode(effects::exile_first_target, "Exile target enchantment.").targets(&[targets::ENCHANTMENT]),
    mode(controller_manifests_dread, "Manifest dread."),
]);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    ..oracle::BREAK_DOWN_THE_DOOR
};
