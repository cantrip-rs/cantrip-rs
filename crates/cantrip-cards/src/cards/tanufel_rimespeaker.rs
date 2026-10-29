use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::YOU_CAST_A_SPELL_WITH_MANA_VALUE_4_OR_GREATER;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_CAST_A_SPELL_WITH_MANA_VALUE_4_OR_GREATER,
        |g, item| effects::draw(g, item, 1),
        "Whenever you cast a spell with mana value 4 or greater, draw a card.",
    )],
    ..oracle::TANUFEL_RIMESPEAKER
};
