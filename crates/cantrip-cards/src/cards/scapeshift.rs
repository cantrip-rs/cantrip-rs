use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[], effects::sacrifice_any_number_of_lands_for_lands), ..oracle::SCAPESHIFT };
