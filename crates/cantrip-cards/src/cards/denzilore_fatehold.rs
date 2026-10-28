use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_SCRY_OR_SURVEIL, effects::plus_one_counter_on_each_creature_you_control, "Whenever you scry or surveil, put a +1/+1 counter on each creature you control.")],
    ..oracle::DENZILORE_FATEHOLD
};
