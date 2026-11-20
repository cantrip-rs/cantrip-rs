use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "When this creature enters, it deals 3 damage to each opponent and you gain 3 life": the life is gained whether or not the
/// damage was dealt (CR 608.2c).
fn drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent_and_gain_life(g, item, 3)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, drain, "When this creature enters, it deals 3 damage to each opponent and you gain 3 life."),
        triggered(THIS_DIES, effects::discard_any_number_then_draw_that_many_plus_one, "When this creature dies, discard any number of cards, then draw that many cards plus one."),
    ],
    ..oracle::COLOSSUS_OF_THE_BLOOD_AGE
};
