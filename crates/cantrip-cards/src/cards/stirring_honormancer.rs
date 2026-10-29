use cantrip_api::*;
use cantrip_mechanics::library::look_at_top_take_one_rest_to_graveyard;
use cantrip_mechanics::{creatures_you_control, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Look at the top X cards of your library, where X is the number of creatures you control": counted as the ability resolves,
/// this creature among them if it is still on the battlefield (CR 608.2h).
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = creatures_you_control(g, item.controller);
    look_at_top_take_one_rest_to_graveyard(g, item.controller, item.object, x)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        look,
        "When this creature enters, look at the top X cards of your library, where X is the number of creatures you control. Put one of those cards into your hand and the rest into your graveyard.",
    )],
    ..oracle::STIRRING_HONORMANCER
};
