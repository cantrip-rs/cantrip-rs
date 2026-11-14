use cantrip_api::*;
use cantrip_mechanics::replacements::GAIN_THAT_MUCH_LIFE_PLUS_ONE;
use cantrip_mechanics::statics::{itself_with_life_at_least, plus_pt};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    replacements: &[GAIN_THAT_MUCH_LIFE_PLUS_ONE],
    statics: &[static_ability(Layer::PtModify, itself_with_life_at_least::<25>, plus_pt::<2, 2>, "This creature gets +2/+2 as long as you have 25 or more life.")],
    ..oracle::ANGEL_OF_VITALITY
};
