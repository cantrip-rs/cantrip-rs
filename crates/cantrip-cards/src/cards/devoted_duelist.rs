use cantrip_api::*;
use cantrip_mechanics::{effects, flurry};
use cantrip_oracle as oracle;

/// Flurry (an ability word, CR 207.2c): the trigger counts the spells its controller cast this turn, those cast
/// before the Duelist entered included, so the Duelist cast as the second spell does not trigger itself.
pub static CARD: CardDef = CardDef {
    abilities: &[flurry::trigger("Whenever you cast your second spell each turn, this creature deals 1 damage to each opponent.", effects::damage_each_opponent_n::<1>)],
    ..oracle::DEVOTED_DUELIST
};
