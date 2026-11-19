use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "Return target creature you don't control with mana value 3 or less to its owner's hand" (CR 400.3,
/// 608.2b): the creature must still be a legal target as it resolves, else the spell does nothing and, not having resolved, goes to
/// the graveyard rather than exile (CR 715.3d).
pub static ENTRY_DENIED: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_AN_OPPONENT_CONTROLS_WITH_MANA_VALUE_3_OR_LESS], effects::return_first_target_to_hand),
    ..oracle::ENTRY_DENIED
};

pub static CARD: CardDef = CardDef { adventure: Some(&ENTRY_DENIED), ..oracle::BELUNAS_GATEKEEPER };

pub static EXTRA: &[&CardDef] = &[&ENTRY_DENIED];
