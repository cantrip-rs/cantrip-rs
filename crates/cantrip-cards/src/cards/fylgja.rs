use cantrip_api::*;
use cantrip_mechanics::{prevention, targets};
use cantrip_oracle as oracle;

/// This Aura enters with four healing counters (CR 614.1c, 122.6).
fn enters_with_healing(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::Healing as usize] = entering.counters[CounterKind::Healing as usize].saturating_add(4);
    Ok(())
}

/// Prevent the next 1 damage to the enchanted creature this turn (CR 608.2h, 615.7).
fn shield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to {
        prevention::this_turn(g, item, Target::Object(creature), 0, Shield::Next(1));
    }
    Ok(())
}

/// Put a healing counter on this Aura (CR 122.1).
fn add_healing(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Healing, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(remove_counters(CounterKind::Healing, 1), shield,
            "Remove a healing counter from this Aura: Prevent the next 1 damage that would be dealt to enchanted creature this turn."),
        ability(cost("{2}{W}"), add_healing, "{2}{W}: Put a healing counter on this Aura."),
    ],
    enters: Some(enters_with_healing),
    enchant: Some(targets::CREATURE),
    granted_preventions: &[prevention::ANY_DAMAGE.text("Prevent the next 1 damage that would be dealt to enchanted creature this turn.")],
    ..oracle::FYLGJA
};
