use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::tokens::{self, ZOMBIE_ROGUE};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "Create a 2/2 blue and black Zombie Rogue creature token."
fn zombie_rogue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ZOMBIE_ROGUE, 1)?;
    Ok(())
}

/// "When this creature enters, if a creature died this turn, create a 2/2 blue and black Zombie Rogue creature token." is an
/// intervening "if" (CR 603.4): any creature's death this turn counts, and it must have happened when the creature enters (or
/// the ability never triggers) and still be so as the ability resolves. "Plot {2}{B}" (CR 702.170) is the shared mechanic.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            zombie_rogue,
            "When this creature enters, if a creature died this turn, create a 2/2 blue and black Zombie Rogue creature token.",
        )
        .condition(history::a_creature_died_this_turn),
        plot(
            mana("{2}{B}"),
            "Plot {2}{B} (You may pay {2}{B} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
    ],
    ..oracle::RICTUS_ROBBER
};

pub static EXTRA: &[&CardDef] = &[&ZOMBIE_ROGUE];
