use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

const SNOW: Supertypes = Supertypes { basic: false, legendary: false, snow: true, world: false };

/// "Target nonsnow basic land" (CR 115.1, 205.4c, 205.4g): a land with the basic supertype and without snow, as the layers
/// make it. A land with a basic land type but not the basic supertype (a dual land) is no basic land (the ruling).
fn nonsnow_basic_land_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::LAND.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if q.chars(id).supertypes.basic && !q.chars(id).supertypes.snow)
}

/// "Target snow land is no longer snow." The land loses the snow supertype for the rest of the game, as long as it stays
/// on the battlefield (CR 611.2a, 400.7), in layer 4 at this effect's timestamp (CR 613.1d), keeping its other supertypes
/// (CR 205.4b).
fn no_longer_snow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, land, EffectKind::LoseSupertypes(SNOW), Duration::Indefinite);
    Ok(())
}

/// "Target nonsnow basic land becomes snow." The land gains the snow supertype for the rest of the game, as long as it
/// stays on the battlefield (CR 611.2a, 400.7), in layer 4 (CR 613.1d); it is still basic (CR 205.4b, the ruling).
fn becomes_snow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, land, EffectKind::AddSupertypes(SNOW), Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}").and(TAP), no_longer_snow, "{2}, {T}: Target snow land is no longer snow.")
            .targets(&[targets::SNOW_LAND]),
        ability(cost("{2}").and(TAP), becomes_snow, "{2}, {T}: Target nonsnow basic land becomes snow.")
            .targets(&[target(nonsnow_basic_land_filter)]),
    ],
    ..oracle::ARCUMS_WEATHERVANE
};
