use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{exile_target_until_leaves, sneak, targets};
use cantrip_oracle as oracle;

/// Exiles the chosen creature until Kunoichi leaves the battlefield (CR 610.3), or for good if its sneak cost was paid.
/// The until-form exiles nothing when Kunoichi already left (CR 610.3b); a token exiled either way is gone (CR 111.7).
fn exile_the_chosen_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if sneak::was_cast_for_sneak(g, item.object) {
        if let Some(creature) = targets::object(g, item, 0) {
            g.exile(creature);
        }
    } else {
        exile_target_until_leaves(g, item, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ways: &[sneak::way(mana("{2}{W}{B}"), "Sneak {2}{W}{B}")],
    abilities: &[triggered(THIS_ENTERS, exile_the_chosen_creature, "When this creature enters, choose target creature an opponent controls. Exile that creature until this creature leaves the battlefield. If this creature's sneak cost was paid, instead exile the chosen creature.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::TURNCOAT_KUNOICHI
};
