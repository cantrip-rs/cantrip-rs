use cantrip_api::*;
use cantrip_mechanics::{Animation, animate};
use cantrip_oracle as oracle;

/// "Put seven +1/+1 counters on this artifact. It becomes a 0/0 Spirit creature in addition to its other types." The counters
/// are put on first (CR 122.1) and then, as the next instruction (CR 608.2c), it becomes a creature (CR 205.1b, 613.1d) with the
/// Spirit type, read through the words of this card's text (CR 612) and base power and toughness 0/0 (CR 613.4b), so the
/// counters count on top: a 7/7. The text names no duration (CR 611.2a): it lasts until the artifact leaves the battlefield
/// (CR 400.7). Both parts are about this artifact alone, not a new object once it has left.
fn seven_counters_and_becomes_a_spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 7);
    let animation = Animation {
        power: 0,
        toughness: 0,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[g.words(item.object).subtype(Subtype::Spirit)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::NONE,
        ability: None,
    };
    animate(g, item.object, item.object, &animation, Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_one_of(&[Color::White, Color::Black]), "{T}: Add {W} or {B}."),
        mana_ability(TAP.and(life(1)), add_one_of(&[Color::Green, Color::Blue, Color::Red]), "{T}, Pay 1 life: Add {G}, {U}, or {R}."),
        ability(
            cost("{7}"),
            seven_counters_and_becomes_a_spirit,
            "{7}: Put seven +1/+1 counters on this artifact. It becomes a 0/0 Spirit creature in addition to its other types. Activate only once.",
        )
        .once(),
    ],
    ..oracle::HAUNTED_SCREEN
};
