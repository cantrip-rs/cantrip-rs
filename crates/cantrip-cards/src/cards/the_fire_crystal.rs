use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, delayed};
use cantrip_mechanics::statics::{have_haste, your_creatures};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Create a token that's a copy of target creature you control. Sacrifice it at the beginning of the next end step" (CR 707.2,
/// 111.1, 603.7): the delayed ability is about exactly the token this resolution made (CR 603.7c). Nothing happens if the target
/// is no longer legal (CR 608.2b).
fn copy_and_sacrifice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let tokens = g.create_token_copies(creature, item.controller, 1)?;
    delayed::at_next_end_step_about(g, item, 0, &tokens);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cost_changes: &[cost_reduction::spells_of_color_you_cast_cost_one_less::<{ Color::Red as u8 }>],
    statics: &[static_ability(Layer::Ability, your_creatures, have_haste, "Creatures you control have haste.")],
    abilities: &[ability(cost("{4}{R}{R}, {T}"), copy_and_sacrifice, "{4}{R}{R}, {T}: Create a token that's a copy of target creature you control. Sacrifice it at the beginning of the next end step.")
        .targets(&[targets::CREATURE_YOU_CONTROL])],
    delayed: &[delayed_trigger(delayed::sacrifice_created, "Sacrifice it at the beginning of the next end step.")],
    ..oracle::THE_FIRE_CRYSTAL
};
