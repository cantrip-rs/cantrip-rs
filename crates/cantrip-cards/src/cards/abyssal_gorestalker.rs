use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, each player sacrifices two creatures of their choice" (CR 701.21a, 101.4): the Gorestalker is on the battlefield as the
/// trigger resolves, so its controller may sacrifice it too, and a player with fewer than two creatures sacrifices all they have.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::each_player_sacrifices_creatures::<2>,
        "When this creature enters, each player sacrifices two creatures of their choice.",
    )],
    ..oracle::ABYSSAL_GORESTALKER
};
