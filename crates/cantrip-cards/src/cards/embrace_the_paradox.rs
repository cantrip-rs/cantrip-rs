use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Draw three cards. You may put a land card from your hand onto the battlefield tapped." (CR 121.1, 608.2d, 614.1d): the
/// cards just drawn are in the hand to choose from; nothing is asked with no land card. Putting a land onto the battlefield
/// is not playing it, so it uses no land play (CR 305.2).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 3)?;
    g.end_sentence();
    effects::may_put_a_land_from_hand_tapped(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::EMBRACE_THE_PARADOX };
