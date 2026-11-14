use cantrip_api::*;
use cantrip_mechanics::replacements::EXILE_INSTEAD_OF_DYING;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::damage_target_exile_if_it_would_die::<3>, "Suplex deals 3 damage to target creature. If that creature would die this turn, exile it instead.")
        .targets(&[targets::CREATURE]),
    mode(effects::exile_first_target, "Exile target artifact.")
        .targets(&[targets::ARTIFACT]),
]);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If that creature would die this turn, exile it instead.")],
    ..oracle::SUPLEX
};
