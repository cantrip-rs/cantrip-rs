use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Add that much {R}. Until end of turn, you don't lose this mana as steps and phases end" (CR 106.4, 500.5a): the damage
/// dealt, however many sources dealt it at once (`sums_per_subject`).
fn add_that_much_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana_until_end_of_turn(item.controller, Mana::R, item.data[0] as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DEALT_DAMAGE,
        add_that_much_red,
        "Enrage — Whenever Raphael is dealt damage, add that much {R}. Until end of turn, you don't lose this mana as steps and phases end.",
    )
    .sums_per_subject()],
    // "Raphael must be blocked if able" (CR 509.1c): the attacker it names is itself.
    must_be_blocked: &[|_, source, attacker| attacker == source],
    ..oracle::RAPHAEL_NINJA_DESTROYER
};
