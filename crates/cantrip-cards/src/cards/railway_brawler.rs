use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Put X +1/+1 counters on it, where X is its power." (CR 122.1a, 608.2h): the entering creature is the trigger's subject, and X is the
/// power the layers give it as the ability resolves; a creature that has left the battlefield by now is a new object (CR 400.7), so it
/// gets nothing, and a power of zero or less puts none on.
fn counters_equal_to_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject
        && g.is_on_battlefield(creature)
    {
        let power = g.chars(creature).power.unwrap_or(0).max(0);
        g.add_counters(creature, CounterKind::PlusOne, power);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
            counters_equal_to_its_power,
            "Whenever another creature you control enters, put X +1/+1 counters on it, where X is its power.",
        ),
        plot(
            mana("{3}{G}"),
            "Plot {3}{G} (You may pay {3}{G} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
    ],
    ..oracle::RAILWAY_BRAWLER
};
