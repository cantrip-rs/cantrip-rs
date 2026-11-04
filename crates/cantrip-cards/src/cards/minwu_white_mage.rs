use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_each_of_subtype_you_control;
use cantrip_mechanics::lands;
use cantrip_mechanics::triggers::YOU_GAIN_LIFE;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_GAIN_LIFE,
        plus_one_counter_on_each_of_subtype_you_control::<{ lands::sub(Subtype::Cleric) }>,
        "Whenever you gain life, put a +1/+1 counter on each Cleric you control.",
    )],
    ..oracle::MINWU_WHITE_MAGE
};
