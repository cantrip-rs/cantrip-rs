use cantrip_api::*;
use cantrip_mechanics::ENTERS_EXILE_UP_TO_ONE_NONLAND_PERMANENT_AN_OPPONENT_CONTROLS_UNTIL_LEAVES_AND_YOU_GAIN_2_LIFE as EXILE_AND_GAIN;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { abilities: &[EXILE_AND_GAIN], ..oracle::PRAYER_OF_BINDING };
