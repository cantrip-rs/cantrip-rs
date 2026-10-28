use cantrip_api::*;
use cantrip_mechanics::{hand, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, draw a card, then you may put a land card from your hand onto the battlefield tapped." (CR 608.2c,
/// 608.2d, 614.1c): the draw ends the sentence before the land is chosen from the hand it left, so the card just drawn may be the
/// one put. Putting a land onto the battlefield is not playing it, so it uses no land play (CR 305.2).
fn draw_put_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    g.end_sentence();
    hand::may_put_onto_battlefield_tapped(g, item.controller, item.object, |g, c| g.chars(c).is(CardType::Land))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        draw_put_a_land,
        "When this creature enters, draw a card, then you may put a land card from your hand onto the battlefield tapped.",
    )],
    ..oracle::H_E_R_B_I_E_SCOUT_UNIT
};
