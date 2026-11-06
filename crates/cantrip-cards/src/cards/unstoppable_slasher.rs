use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT, THIS_DIES};
use cantrip_oracle as oracle;

/// "If it had no counters on it" (CR 603.4, 603.10a): the creature as it last was on the battlefield, counters of every kind.
fn had_no_counters(q: &dyn Query, asker: &Asker) -> bool {
    CounterKind::ALL.iter().all(|&kind| q.obj(asker.source).counters(kind) == 0)
}

/// "Return it to the battlefield tapped under its owner's control with two stun counters on it" (CR 400.7, 122.1d): the card
/// it became in the graveyard, if it is still there - each card of a merged permanent, together (CR 730.3c).
fn return_stunned(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_what_it_became(g, item.object, true, &[(CounterKind::Stun, 2)])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
            effects::subject_player_loses_half_their_life,
            "Whenever this creature deals combat damage to a player, they lose half their life, rounded up.",
        ),
        triggered(
            THIS_DIES,
            return_stunned,
            "When this creature dies, if it had no counters on it, return it to the battlefield tapped under its owner's control with two stun counters on it.",
        )
        .condition(had_no_counters),
    ],
    ..oracle::UNSTOPPABLE_SLASHER
};
