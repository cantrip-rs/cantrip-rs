use cantrip_api::*;
use cantrip_mechanics::{effects, prevention};
use cantrip_oracle as oracle;

/// Put a +1/+0 counter on this creature (CR 122.1).
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Plus1Plus0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(remove_counters(CounterKind::Plus1Plus0, 1), prevention::next_1_damage_to_this_this_turn,
            "Remove a +1/+0 counter from this creature: Prevent the next 1 damage that would be dealt to it this turn."),
        ability(cost("{R}{R}{R}"), grow,
            "{R}{R}{R}: Put a +1/+0 counter on this creature. Activate only during your upkeep.")
            .condition(cantrip_mechanics::your_upkeep),
    ],
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::Plus1Plus0 as usize }>),
    granted_preventions: &[prevention::ANY_DAMAGE.text("Prevent the next 1 damage that would be dealt to it this turn.")],
    ..oracle::BALDUVIAN_HYDRA
};
