use cantrip_api::*;
use cantrip_mechanics::{effects, prevention};
use cantrip_oracle as oracle;

/// Prevent the next 1 damage dealt to this creature this turn (CR 615.7).
fn shield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::this_turn(g, item, Target::Object(item.object), 0, Shield::Next(1));
    Ok(())
}

/// Put a +1/+0 counter on this creature (CR 122.1).
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Plus1Plus0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(remove_counters(CounterKind::Plus1Plus0, 1), shield,
            "Remove a +1/+0 counter from this creature: Prevent the next 1 damage that would be dealt to it this turn."),
        ability(cost("{R}{R}{R}"), grow,
            "{R}{R}{R}: Put a +1/+0 counter on this creature. Activate only during your upkeep.")
            .condition(cantrip_mechanics::your_upkeep),
    ],
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::Plus1Plus0 as usize }>),
    granted_preventions: &[prevention::ANY_DAMAGE.text("Prevent the next 1 damage that would be dealt to it this turn.")],
    ..oracle::BALDUVIAN_HYDRA
};
