use cantrip_api::*;
use cantrip_mechanics::{effects, pump, triggers};
use cantrip_oracle as oracle;

/// "This creature gets +1/+1 until end of turn for each enchantment you control" (CR 611.2a, 611.2c): the enchantments are
/// counted once, as the ability resolves, and the bonus is fixed by that count (CR 608.2h).
fn pump_per_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Enchantment)).count() as i32;
    pump(g, item.object, item.object, count, count);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::ENCHANTMENT_YOU_CONTROL_ENTERS,
            |g, item| effects::scry(g, item, 1),
            "Whenever an enchantment you control enters, scry 1.",
        ),
        ability(
            cost("{2}{W}"),
            pump_per_enchantment,
            "{2}{W}: This creature gets +1/+1 until end of turn for each enchantment you control.",
        ),
    ],
    ..oracle::SLUMBERING_KEEPGUARD
};
