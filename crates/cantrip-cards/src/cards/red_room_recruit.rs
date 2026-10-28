use cantrip_api::*;
use cantrip_mechanics::{connive, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, it connives." (CR 701.50): the Recruit connives even if it has already left the battlefield.
fn connives(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    connive(g, item.object, item.object)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, connives, "When this creature enters, it connives.")],
    ..oracle::RED_ROOM_RECRUIT
};
