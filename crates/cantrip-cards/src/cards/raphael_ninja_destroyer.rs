use cantrip_api::*;
use cantrip_mechanics::{combat, triggers};
use cantrip_oracle as oracle;

/// "Add that much {R}. Until end of turn, you don't lose this mana as steps and phases end" (CR 106.4, 500.5a): the damage
/// dealt, however many sources dealt it at once (`sums_per_subject`).
fn add_that_much_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana_until_end_of_turn(item.controller, Mana::R, item.matched_damage_amount());
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_TAKES_DAMAGE,
        add_that_much_red,
        "Enrage — Whenever Raphael is dealt damage, add that much {R}. Until end of turn, you don't lose this mana as steps and phases end.",
    ).damage_amount()
    .produces_mana().sums_per_subject()],
    must_be_blocked: &[combat::this_must_be_blocked_if_able],
    ..oracle::RAPHAEL_NINJA_DESTROYER
};
