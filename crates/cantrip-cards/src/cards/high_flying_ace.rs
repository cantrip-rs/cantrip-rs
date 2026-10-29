use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature without flying" (CR 115.1, 702.9): judged on the creature's keywords as it is chosen and again as the
/// ability resolves (CR 608.2b).
fn creature_without_flying(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) && matches!(t, Target::Object(id) if !q.chars(id).has(Keyword::Flying))
}

/// "Target creature without flying gains flying until end of turn."
fn gain_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{3}{W}"), gain_flying, "{3}{W}: Target creature without flying gains flying until end of turn. Activate only as a sorcery.")
        .targets(&[target(creature_without_flying)])
        .sorcery_speed()],
    ..oracle::HIGH_FLYING_ACE
};
