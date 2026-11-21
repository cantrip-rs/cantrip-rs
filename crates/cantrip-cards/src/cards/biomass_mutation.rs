use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::creatures_you_control_have_base_pt_x),
    ..oracle::BIOMASS_MUTATION
};
