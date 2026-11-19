use cantrip_api::*;
use cantrip_mechanics::statics::{INSTANT_OR_SORCERY, cant_be_countered, your_spells_of_types};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, your_spells_of_types::<INSTANT_OR_SORCERY>, cant_be_countered, "Instant and sorcery spells you control can't be countered.").everywhere()],
    ..oracle::SPHINX_OF_THE_FINAL_WORD
};
