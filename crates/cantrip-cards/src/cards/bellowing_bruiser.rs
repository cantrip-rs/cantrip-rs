use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "Up to two target creatures can't block this turn."
pub static BEAT_A_PATH: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.between(0, 2)], effects::each_target_cant_block_this_turn),
    ..oracle::BEAT_A_PATH
};

pub static CARD: CardDef = CardDef { adventure: Some(&BEAT_A_PATH), ..oracle::BELLOWING_BRUISER };

pub static EXTRA: &[&CardDef] = &[&BEAT_A_PATH];
