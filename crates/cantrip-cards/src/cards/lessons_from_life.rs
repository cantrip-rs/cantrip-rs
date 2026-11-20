use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Draw three cards. You may put a land card from your hand onto the battlefield tapped." (CR 608.2c): the draw ends the
/// sentence before the land is chosen from the hand it left, so a land just drawn can be the one chosen; any land card of the hand may
/// be chosen, or none. Putting a land onto the battlefield is not playing it, so it uses no land play (CR 305.4).
fn draw_three_put_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 3);
    effects::may_put_a_land_from_hand_tapped(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], draw_three_put_a_land), ..oracle::LESSONS_FROM_LIFE };
