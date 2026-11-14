use cantrip_api::*;
use cantrip_mechanics::replacements::EXILE_INSTEAD_OF_DYING;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_PLANESWALKER], effects::damage_target_exile_if_it_would_die::<3>),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If that creature or planeswalker would die this turn, exile it instead.")],
    ..oracle::SCORCHING_DRAGONFIRE
};
