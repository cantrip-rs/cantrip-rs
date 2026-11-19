use cantrip_api::*;
use cantrip_mechanics::{discover, targets};
use cantrip_oracle as oracle;

/// "Target artifact or creature with mana value 1 or greater" (CR 115.1, 202.3): a token copy of a card with no mana cost is no
/// legal target.
fn artifact_or_creature_with_mana_value_one_or_greater(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::artifact_or_creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.mana_value(id) >= 1)
}

static TARGETS: &[TargetSpec] = &[target(artifact_or_creature_with_mana_value_one_or_greater)];

/// "The owner of target artifact or creature with mana value 1 or greater shuffles it into their library. Then that player
/// discovers X, where X is its mana value" (CR 701.24, 701.57a, 608.2h): the owner, not the permanent's controller, does both. X
/// is the mana value the permanent had on the battlefield, read before it leaves (CR 608.2h, last known information).
fn shuffle_into_the_owners_library_then_the_owner_discovers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let owner = g.state().obj(permanent).owner;
    let x = g.mana_value(permanent);
    g.put_into_library(&[permanent], LibraryPlace::ShuffledIn);
    discover::discover(g, owner, x)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(TARGETS, shuffle_into_the_owners_library_then_the_owner_discovers),
    ..oracle::ZOYOWAS_JUSTICE
};
