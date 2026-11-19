use cantrip_api::*;
use cantrip_mechanics::statics::{INSTANT_OR_SORCERY, have_lifelink, your_spells_of_types};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Heartflame Slash.
pub static HEARTFLAME_SLASH: CardDef = CardDef { spell: spell(&[targets::ANY], effects::damage_first_target::<3>), ..oracle::HEARTFLAME_SLASH };

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        your_spells_of_types::<INSTANT_OR_SORCERY>,
        have_lifelink,
        "Instant and sorcery spells you control have lifelink.",
    )
    .everywhere()],
    adventure: Some(&HEARTFLAME_SLASH),
    ..oracle::HEARTFLAME_DUELIST
};

pub static EXTRA: &[&CardDef] = &[&HEARTFLAME_SLASH];
