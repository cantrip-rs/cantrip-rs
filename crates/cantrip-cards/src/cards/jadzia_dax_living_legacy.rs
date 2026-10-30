use cantrip_api::*;
use cantrip_oracle as oracle;

use cantrip_mechanics::{effects, targets, triggers};

/// "Put a +1/+1 counter on Jadzia, then add {G} for each counter on her." (CR 122.1, 106.1): counters of every kind, counted after the counter is added.
fn first_main(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    g.end_sentence();
    let counters = effects::counters_on(g, item.object);
    g.add_mana(item.controller, Mana::G, counters);
    Ok(())
}

/// "Put two +1/+1 counters on target creature." (CR 122.1, 601.2c): only resolves for a still-legal target.
fn transfer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        g.add_counters(creature, CounterKind::PlusOne, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, first_main, "At the beginning of your first main phase, put a +1/+1 counter on Jadzia Dax, then add an amount of {G} equal to the number of counters on her."),
        ability(cost("{2}{G}").and(DISCARD_THIS), transfer, "Transfer Symbiont — {2}{G}, Discard this card: Put two +1/+1 counters on target creature.")
            .targets(&[targets::CREATURE])
            .from(ZoneKind::Hand),
    ],
    ..oracle::JADZIA_DAX_LIVING_LEGACY
};
