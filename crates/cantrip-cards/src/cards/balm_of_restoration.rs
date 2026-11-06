use cantrip_api::*;
use cantrip_mechanics::{prevention, targets};
use cantrip_oracle as oracle;

/// "You gain 2 life" (CR 119.3).
fn gain_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 2);
    Ok(())
}

/// "Prevent the next 2 damage that would be dealt to any target this turn" (CR 615.7).
fn prevent_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::next_damage_to_target(g, item, 0, 0, 2);
    Ok(())
}

static MODES: ModalDef = modes(&[
    mode(gain_two, "You gain 2 life."),
    mode(prevent_two, "Prevent the next 2 damage that would be dealt to any target this turn.")
        .targets(&[targets::ANY]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_ability(
        cost("{1}").and(SACRIFICE_THIS).and(TAP),
        &MODES,
        "{1}, {T}, Sacrifice this artifact: Choose one — You gain 2 life; or Prevent the next 2 damage that would be dealt to any target this turn.",
    )],
    granted_preventions: &[prevention::ANY_DAMAGE.text("Prevent the next 2 damage that would be dealt to any target this turn.")],
    ..oracle::BALM_OF_RESTORATION
};
