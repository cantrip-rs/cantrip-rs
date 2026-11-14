use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Put X +1/+1 counters on this creature, where X is its power" (CR 122.1, 107.3c): X is this creature's power as the ability
/// resolves, counters included. Once it has left the battlefield it is a new object (CR 400.7) and none are put on it.
fn put_x_counters_where_x_is_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !is_live(g, item.object) {
        return Ok(());
    }
    // X of 0 or less puts none on (CR 107.1b, 122.1).
    if let Some(x) = g.chars(item.object).power.filter(|&x| x > 0) {
        g.add_counters(item.object, CounterKind::PlusOne, x);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{G}{G}"),
        put_x_counters_where_x_is_its_power,
        "{2}{G}{G}: Put X +1/+1 counters on this creature, where X is its power.",
    )],
    enters: Some(effects::enters_with_plus_one_counters::<4>),
    ..oracle::HEROES_BANE
};
