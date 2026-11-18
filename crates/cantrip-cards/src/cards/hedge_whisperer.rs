use cantrip_api::*;
use cantrip_mechanics::{animate, targets, Animation};
use cantrip_oracle as oracle;

/// "Target land you control becomes a 5/5 green Plant Boar creature with haste for as long as this creature remains tapped. It's
/// still a land." (CR 611.2b, 205.1b): the land keeps its types and subtypes and gains Creature, Plant and Boar, its colors
/// become green, and it is a 5/5 with haste for as long as this creature is on the battlefield and tapped, from the final
/// {T} cost: a Whisperer that left (or untapped) before this resolves leaves the land as it was.
fn animate_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    animate(
        g,
        item.object,
        land,
        &Animation {
            power: 5,
            toughness: 5,
            colors: Some(ColorSet::of(Color::Green)),
            types: TypeSet::NONE,
            subtypes: &[Subtype::Plant, Subtype::Boar],
            all_creature_types: false,
            replaces_creature_types: false,
            keywords: Keywords::of(&[Keyword::Haste]),
            ability: None,
        },
        Duration::WhileSourceTapped,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{G}, {T}").and(Cost { collect_evidence: 4, ..Cost::NONE }),
        animate_land,
        "{3}{G}, {T}, Collect evidence 4: Target land you control becomes a 5/5 green Plant Boar creature with haste for as long as this creature remains tapped. It's still a land. Activate only as a sorcery.",
    )
    .targets(&[targets::LAND_YOU_CONTROL])
    .sorcery_speed()],
    may_not_untap: true,
    ..oracle::HEDGE_WHISPERER
};
