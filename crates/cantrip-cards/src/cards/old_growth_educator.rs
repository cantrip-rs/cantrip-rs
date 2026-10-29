use cantrip_api::*;
use cantrip_mechanics::history::gained_life_this_turn;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Put two +1/+1 counters on it if you gained life this turn": it always triggers, and whether life was gained is asked as it
/// resolves (CR 608.2c), not as it triggers.
fn two_counters_if_life_was_gained(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if gained_life_this_turn(&*g, item.controller) {
        effects::two_plus_one_counters_on_this(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        two_counters_if_life_was_gained,
        "Infusion \u{2014} When this creature enters, put two +1/+1 counters on it if you gained life this turn.",
    )],
    ..oracle::OLD_GROWTH_EDUCATOR
};
