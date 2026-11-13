use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "You draw a card", by the spell's controller.
fn draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], delayed::target_creature_gets_plus_power_and_a_delayed_ability_for_when_it_dies::<2>),
    delayed: &[triggered(triggers::THIS_DIES, draw, "When that creature dies this turn, you draw a card.")],
    ..oracle::A_GOOD_DAY_TO_DIE
};
