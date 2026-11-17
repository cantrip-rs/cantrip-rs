use cantrip_api::*;
use cantrip_mechanics::{effects, spree, targets};
use cantrip_oracle as oracle;

const MODES: &[ModeDef] = &[
    mode(effects::damage_first_target::<4>, "+ {2} \u{2014} Explosive Derailment deals 4 damage to target creature.")
        .targets(&[targets::CREATURE])
        .cost(mana("{2}")),
    mode(effects::destroy_first_target, "+ {2} \u{2014} Destroy target artifact.")
        .targets(&[targets::ARTIFACT])
        .cost(mana("{2}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::EXPLOSIVE_DERAILMENT };
