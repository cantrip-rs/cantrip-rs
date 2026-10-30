use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { abilities: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")], ..oracle::GREAT_FOREST_DRUID };
