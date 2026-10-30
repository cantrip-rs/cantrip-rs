use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_by(mana::add_one_of_each_color_among_your_permanents), "Vivid \u{2014} {T}: For each color among permanents you control, add one mana of that color.")],
    ..oracle::BLOOM_TENDER
};
