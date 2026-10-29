use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Add two mana of any one color." The color is chosen as the ability resolves (CR 106.1b, 106.4).
fn add_two_of_one_color(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let color =
        cantrip_mechanics::choose_color_for(g, item.controller, item.object, ColorUse::AddMana)?;
    g.add_mana(item.controller, color.mana(), 2);
    Ok(())
}

/// "When Culber and Stamets enter, choose one — Put a +1/+1 counter on target creature; or add two mana of any one
/// color." The controller chooses a legal mode as the trigger is put on the stack (CR 603.3c, 700.2b); the selected mode
/// alone has its target (CR 700.2c).
static MODAL: ModalDef = modes(&[
    mode(
        effects::plus_one_counter_on_target,
        "Put a +1/+1 counter on target creature.",
    )
    .targets(&[targets::CREATURE]),
    mode(add_two_of_one_color, "Add two mana of any one color."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When Culber and Stamets enter, choose one — Put a +1/+1 counter on target creature; or add two mana of any one color.",
    )],
    ..oracle::CULBER_AND_STAMETS_STAR_CROSSED
};
