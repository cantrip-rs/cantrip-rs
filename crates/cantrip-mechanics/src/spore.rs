//! The spore counter cycle the Fungus and Thallid creatures of Fallen Empires print (CR 122.1, 603.2).

use cantrip_api::*;
use crate::{tokens, triggers};

fn put_spore_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Spore, 1);
    Ok(())
}

/// "At the beginning of your upkeep, put a spore counter on this creature." Elvish Farmer, Feral Thallid, Spore Flower, Thallid,
/// Thallid Devourer and Thorn Thallid print it.
pub const UPKEEP: AbilityDef = triggered(
    triggers::YOUR_UPKEEP,
    put_spore_counter,
    "At the beginning of your upkeep, put a spore counter on this creature.",
);

/// "Remove three spore counters from this creature: Create a 1/1 green Saproling creature token." The token is made with the
/// words of its source's text (CR 612). Elvish Farmer, Thallid and Thallid Devourer print it; the card lists
/// `tokens::SAPROLING` in its `EXTRA`.
pub const SAPROLING: AbilityDef = ability(
    remove_counters(CounterKind::Spore, 3),
    tokens::create_a_saproling,
    "Remove three spore counters from this creature: Create a 1/1 green Saproling creature token.",
);
