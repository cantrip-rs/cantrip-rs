use cantrip_api::*;
use cantrip_mechanics::effects::may_put_a_card_from_hand_on_the_bottom_to_draw;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Volcanic Spite deals 3 damage to target creature, planeswalker, or battle." (CR 120.3) Then the shared "you may put a card
/// from your hand on the bottom of your library. If you do, draw a card."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 3);
    g.end_sentence();
    may_put_a_card_from_hand_on_the_bottom_to_draw(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_PLANESWALKER_OR_BATTLE], effect), ..oracle::VOLCANIC_SPITE };
