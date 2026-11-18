use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return it to the battlefield under its owner's control and suspect it" (CR 110.2a, 400.7, 701.60a): the card the creature
/// became in the graveyard returns untapped, and what actually entered is suspected - a token has ceased to exist and returns
/// nothing, so nothing is suspected.
fn return_it_and_suspect_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let returned = effects::return_what_it_became(g, item.object, false, &[])?;
    for creature in returned {
        g.suspect(creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_and_gains_dies_trigger::<2, 0, 0>),
    delayed: &[triggered(
        THIS_DIES,
        return_it_and_suspect_it,
        "When this creature dies, return it to the battlefield under its owner's control and suspect it.",
    )],
    ..oracle::PRESUMED_DEAD
};
