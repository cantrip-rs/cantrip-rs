use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature and add {R}."
fn counter_and_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    g.add_mana(item.controller, Mana::R, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        // A triggered mana ability (CR 605.1b) where the sacrifice pays a mana ability's cost (a Treasure's, CR 605.4a): it
        // resolves at once, with no stack. Triggered by anything else, it is an ordinary triggered ability that adds its
        // mana when it resolves. The advice cannot say which a sacrifice will be, so it counts no mana for it.
        triggered(
            triggers::YOU_SACRIFICE_AN_ARTIFACT,
            counter_and_red,
            "Whenever you sacrifice an artifact, put a +1/+1 counter on this creature and add {R}.",
        )
        .on_sacrifice()
        .mana_ability(TriggeredMana::Unstated),
    ],
    ..oracle::CRIME_NOVELIST
};
