use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, effects::draw_a_card_and_lose_1_life, "Whenever this creature attacks, you draw a card and lose 1 life.")],
    ..oracle::MOONGLOVE_EXTRACTOR
};
