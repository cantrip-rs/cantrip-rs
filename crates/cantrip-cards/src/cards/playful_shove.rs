use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// "Playful Shove deals 1 damage to any target. Draw a card." (CR 120.3, 121.1) A spell whose only target is illegal does not
/// resolve at all (CR 608.2b), so nothing is drawn then.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 1);
    g.end_sentence();
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::ANY], effect), ..oracle::PLAYFUL_SHOVE };
