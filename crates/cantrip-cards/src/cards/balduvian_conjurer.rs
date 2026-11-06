use cantrip_api::*;
use cantrip_mechanics::{animate, targets, Animation};
use cantrip_oracle as oracle;

/// A snow land permanent (CR 115.1).
fn snow_land(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::LAND.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).supertypes.snow)
}

/// The land becomes a 2/2 creature while keeping its other characteristics (CR 205.1b, 611.2a).
fn animate_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    animate(g, item.object, land, &Animation {
            power: 2,
            toughness: 2,
            colors: None,
            types: TypeSet::NONE,
            subtypes: &[],
            all_creature_types: false,
            replaces_creature_types: false,
            keywords: Keywords::NONE,
            ability: None,
        }, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, animate_land,
        "{T}: Target snow land becomes a 2/2 creature until end of turn. It's still a land.")
        .targets(&[target(snow_land)])],
    ..oracle::BALDUVIAN_CONJURER
};
