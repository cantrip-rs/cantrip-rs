use cantrip_api::*;
use cantrip_mechanics::replacements::EXILE_INSTEAD_OF_DYING;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::damage_target_exile_if_it_would_die::<5>),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If that creature would die this turn, exile it instead.")],
    ..oracle::FEED_THE_FLAMES
};
