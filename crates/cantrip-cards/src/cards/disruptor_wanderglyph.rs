use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, effects::exile_first_target, "Whenever this creature attacks, exile target card from an opponent's graveyard.")
        .targets(&[targets::CARD_IN_AN_OPPONENTS_GRAVEYARD])],
    ..oracle::DISRUPTOR_WANDERGLYPH
};
