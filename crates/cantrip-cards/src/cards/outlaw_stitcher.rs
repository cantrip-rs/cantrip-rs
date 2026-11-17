use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, create a 2/2 blue and black Zombie Rogue creature token, then put two +1/+1 counters on that token for each
/// spell you've cast this turn other than the first" (CR 111.1, 122.1, 603.6a): the spells counted are asked of the event history as the
/// ability resolves, so a spell that was countered or is still on the stack counts, as does the Stitcher itself when it was cast and
/// was not the first (the ruling). The counters go on after the token is created, as a second step, so the token enters as a 2/2 first.
/// A copy of a spell is not cast (CR 707.10), and an Outlaw Stitcher put onto the battlefield without being cast counts what was cast.
fn zombie_with_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let created = tokens::create(g, item.controller, &tokens::ZOMBIE_ROGUE, 1)?;
    g.end_sentence();
    let others = g.history().casts_by(item.controller).count().saturating_sub(1) as i32;
    let counters = 2 * others;
    for token in created {
        g.add_counters(token, CounterKind::PlusOne, counters);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            zombie_with_counters,
            "When this creature enters, create a 2/2 blue and black Zombie Rogue creature token, then put two +1/+1 counters on that token for each spell you've cast this turn other than the first.",
        ),
        plot(
            mana("{4}{U}"),
            "Plot {4}{U} (You may pay {4}{U} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
    ],
    ..oracle::OUTLAW_STITCHER
};

pub static EXTRA: &[&CardDef] = &[&tokens::ZOMBIE_ROGUE];
