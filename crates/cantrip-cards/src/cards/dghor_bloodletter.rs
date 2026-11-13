use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(cantrip_mechanics::effects::other_creatures_get::<-1, -1>, "All other creatures get -1/-1 until end of turn."),
    mode(effects::each_player_sacrifices_creatures::<1>, "Each player sacrifices a creature of their choice."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose one —\n• All other creatures get -1/-1 until end of turn.\n• Each player sacrifices a creature of their choice.",
    )],
    ..oracle::DGHOR_BLOODLETTER
};
