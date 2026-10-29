use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// "Tweeze deals 3 damage to any target. You may discard a card. If you do, draw a card." (CR 608.2c): the loot follows the
/// damage; a spell whose only target became illegal does nothing at all (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 3);
    effects::may_discard_to_draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ANY], effect),
    ..oracle::TWEEZE
};
