use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::destroy_all_creatures_and_gain_life_for_each_destroyed),
    ..oracle::FUMIGATE
};
