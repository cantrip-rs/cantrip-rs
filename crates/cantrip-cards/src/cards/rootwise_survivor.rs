use cantrip_api::*;
use cantrip_mechanics::{Animation, animate, targets, triggers};
use cantrip_oracle as oracle;

/// Survival: "Put three +1/+1 counters on up to one target land you control. That land becomes a 0/0 Elemental creature in
/// addition to its other types. It gains haste until your next turn." Three sentences, each its own instruction (CR 608.2c):
/// the counters, the permanent change with no duration (CR 611.2a, 613.1d: it stays a creature; the 0/0 is set in layer 7b so the
/// counters count on top), and the haste until the start of the controller's next turn (CR 611.2a, 613.1f). Nothing happens if the
/// land was not chosen or is no longer a legal target as the ability resolves (CR 608.2b).
fn counters_and_animates_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(land) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(land, CounterKind::PlusOne, 3);
    g.end_sentence();
    let animation = Animation {
        power: 0,
        toughness: 0,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[g.words(item.object).subtype(Subtype::Elemental)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::NONE,
        ability: None,
    };
    animate(g, item.object, land, &animation, Duration::Indefinite);
    g.end_sentence();
    g.add_effect(item.object, land, EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])), Duration::UntilNextTurnOf(item.controller));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::survival(
        counters_and_animates_land,
        "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, put three +1/+1 counters on up to one target land you control. That land becomes a 0/0 Elemental creature in addition to its other types. It gains haste until your next turn.",
    )
    .targets(&[targets::LAND_YOU_CONTROL.optional()])],
    ..oracle::ROOTWISE_SURVIVOR
};
