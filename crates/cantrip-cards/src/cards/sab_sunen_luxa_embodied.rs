use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "It has an even number of counters on it" (CR 122.1; zero is even): counters of every kind, as they are now.
fn has_an_even_number_of_counters(q: &dyn Query, source: ObjectId) -> bool {
    effects::counters_on(q, source) % 2 == 0
}

/// "Sab-Sunen can't attack ... unless it has an even number of counters on it" (CR 508.1c): asked of this creature as attackers are
/// declared. Once declared it keeps attacking whatever happens to its counters (CR 506.4).
fn attacks_only_with_an_even_number_of_counters(q: &dyn Query, asker: &Asker, attacker: ObjectId, _: Target) -> bool {
    asker.source != attacker || has_an_even_number_of_counters(q, asker.source)
}

/// "Sab-Sunen can't ... block unless it has an even number of counters on it" (CR 509.1b).
fn blocks_only_with_an_even_number_of_counters(q: &dyn Query, source: ObjectId, blocker: ObjectId, _: ObjectId) -> bool {
    source != blocker || has_an_even_number_of_counters(q, source)
}

/// "Put a +1/+1 counter on Sab-Sunen. Then if it has an odd number of counters on it, draw two cards" (CR 122.1, 608.2c, 608.2h): two
/// instructions, so the count is taken after the counter is on it; counters of every kind are counted, and an object that has left the
/// battlefield is counted as it last was.
fn grow_then_draw_two_if_odd(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    if effects::counters_on(g, item.object) % 2 == 1 {
        effects::draw(g, item, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_FIRST_MAIN_PHASE,
        grow_then_draw_two_if_odd,
        "At the beginning of your first main phase, put a +1/+1 counter on Sab-Sunen. Then if it has an odd number of counters on it, draw two cards.",
    )],
    attack_rules: &[attacks_only_with_an_even_number_of_counters],
    block_rules: &[blocks_only_with_an_even_number_of_counters],
    ..oracle::SAB_SUNEN_LUXA_EMBODIED
};
