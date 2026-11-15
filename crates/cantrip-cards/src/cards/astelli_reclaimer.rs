use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS, warp};
use cantrip_oracle as oracle;

/// "Target noncreature, nonland permanent card with mana value X or less from your graveyard, where X is the amount of
/// mana spent to cast this creature" (CR 115.1, 110.4b, 202.3, 601.2h): the mana spent is kept on the creature as it was
/// cast, a copy or a creature put onto the battlefield spent none, so X is 0 (its ruling), and a creature that has left
/// the battlefield still has what it was cast with (CR 113.7a). X in a card's mana cost is 0 in a graveyard (CR 202.3e).
fn noncreature_nonland_permanent_card_up_to_mana_spent(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    targets::noncreature_nonland_permanent_card_in_your_graveyard(q, asker, t)
        && q.mana_value(id) <= q.obj(asker.source).spent.total()
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, effects::return_first_target_to_battlefield, "When this creature enters, return target noncreature, nonland permanent card with mana value X or less from your graveyard to the battlefield, where X is the amount of mana spent to cast this creature.")
        .targets(&[target(noncreature_nonland_permanent_card_up_to_mana_spent)])],
    ways: &[warp::way(mana("{2}{W}"), "Warp {2}{W}")],
    end_step_riders: warp::RIDERS,
    delayed: &[warp::EXILE],
    ..oracle::ASTELLI_RECLAIMER
};
