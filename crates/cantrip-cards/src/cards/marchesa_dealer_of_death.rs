use cantrip_api::*;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to look at the top two cards of your library?", "Put one of them into your hand and the other into your graveyard", "Pay {cost}", "Don't pay");

/// "Whenever you commit a crime, you may pay {1}. If you do, look at the top two cards of your library. Put one of them into
/// your hand and the other into your graveyard." (CR 608.2d, 118.12, 701.20e): the payment is made as the ability resolves.
fn pay_to_look_at_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{1}"), &PAY)? {
        library::look_at_top_take_one_rest_to_graveyard(g, item.controller, item.object, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_COMMIT_A_CRIME,
        pay_to_look_at_two,
        "Whenever you commit a crime, you may pay {1}. If you do, look at the top two cards of your library. Put one of them into your hand and the other into your graveyard. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)",
    )],
    ..oracle::MARCHESA_DEALER_OF_DEATH
};
