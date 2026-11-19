use cantrip_api::*;
use cantrip_mechanics::{Animation, animate, targets};
use cantrip_oracle as oracle;

/// "{5}{G}{G}: Put seven +1/+1 counters on target noncreature land you control. It becomes a 0/0 Fungus creature with haste. It's still a
/// land." It states no duration (CR 611.2a), so the land stays a creature: the type added rather than set (CR 205.1b), and the base 0/0 in
/// layer 7b under the counters of 7c. The Fungus subtype is read through the words of this card's text (CR 612.2).
fn animate_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(land, CounterKind::PlusOne, 7);
    g.end_sentence();
    let animation = Animation {
        power: 0,
        toughness: 0,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[g.words(item.object).subtype(Subtype::Fungus)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::of(&[Keyword::Haste]),
        ability: None,
    };
    animate(g, item.object, land, &animation, Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        Cost::mana(mana("{5}{G}{G}")),
        animate_land,
        "{5}{G}{G}: Put seven +1/+1 counters on target noncreature land you control. It becomes a 0/0 Fungus creature with haste. It's still a land.",
    )
    .targets(&[target(targets::noncreature_land_you_control)])],
    ..oracle::TENDRIL_OF_THE_MYCOTYRANT
};
