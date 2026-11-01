use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Another target Ally you control" (CR 115.1, 205.3m): a creature on the battlefield under your control that is an Ally,
/// other than the source.
fn another_ally_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::other_creature_you_control_filter(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Ally)))
}

/// "Another target Ally you control gains indestructible until end of turn." (CR 702.12b, 611.2a)
fn ally_gains_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(ally) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_indestructible_until_end_of_turn(g, item.object, ally);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        SACRIFICE_THIS,
        ally_gains_indestructible,
        "Sacrifice this creature: Another target Ally you control gains indestructible until end of turn.",
    )
    .targets(&[target(another_ally_you_control)])],
    ..oracle::EARTH_KINGDOM_PROTECTORS
};
