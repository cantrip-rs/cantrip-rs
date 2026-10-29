use cantrip_api::*;
use cantrip_mechanics::effects::may_put_a_card_from_hand_on_the_bottom_to_draw;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "Manhole Missile deals 3 damage to target creature." (CR 120.3) Then "you may put a card from your hand on the bottom of
/// your library. If you do, draw a card." A spell whose only target is illegal does not resolve at all (CR 608.2b, the ruling).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 3);
    may_put_a_card_from_hand_on_the_bottom_to_draw(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::MANHOLE_MISSILE };
