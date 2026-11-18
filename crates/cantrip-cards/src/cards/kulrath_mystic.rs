use cantrip_api::*;
use cantrip_mechanics::effects::this_gets_and_gains_vigilance;
use cantrip_mechanics::triggers::YOU_CAST_A_SPELL_WITH_MANA_VALUE_4_OR_GREATER;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_CAST_A_SPELL_WITH_MANA_VALUE_4_OR_GREATER,
        this_gets_and_gains_vigilance::<2, 0>,
        "Whenever you cast a spell with mana value 4 or greater, this creature gets +2/+0 and gains vigilance until end of turn.",
    )],
    ..oracle::KULRATH_MYSTIC
};
