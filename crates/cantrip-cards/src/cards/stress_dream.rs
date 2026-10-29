use cantrip_api::*;
use cantrip_mechanics::{library::look_at_top_two_take_one_other_on_bottom, targets};
use cantrip_oracle as oracle;

/// "Stress Dream deals 5 damage to up to one target creature. Look at the top two cards of your library. Put one of those cards into
/// your hand and the other on the bottom of your library." With no target chosen the spell does the rest; if the one target chosen
/// is illegal as it resolves the whole spell does nothing (CR 608.2b), the look included.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.damage(item.object, Target::Object(creature), 5);
        g.end_sentence();
    }
    look_at_top_two_take_one_other_on_bottom(g, item.controller, item.object)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE.between(0, 1)], effect), ..oracle::STRESS_DREAM };
