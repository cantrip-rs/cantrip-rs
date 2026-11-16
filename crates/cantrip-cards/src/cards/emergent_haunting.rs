use cantrip_api::*;
use cantrip_mechanics::{Animation, animate, effects, history::you_havent_cast_a_spell_from_your_hand, triggers::YOUR_END_STEP};
use cantrip_oracle as oracle;

/// "If you haven't cast a spell from your hand this turn and this enchantment isn't a creature" (CR 603.4): the intervening
/// "if", checked when the end step begins and again as the ability resolves (the rulings).
fn no_spell_from_hand_and_not_a_creature(q: &dyn Query, asker: &Asker) -> bool {
    you_havent_cast_a_spell_from_your_hand(q, asker) && !q.chars(asker.source).is(CardType::Creature)
}

/// "It becomes a 3/3 Spirit creature with flying in addition to its other types": no duration, so it stays one while it stays
/// on the battlefield (CR 611.2a). The text names no colors, so they are not changed.
fn becomes_spirit_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.object;
    let animation = Animation {
        power: 3,
        toughness: 3,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[g.words(me).subtype(Subtype::Spirit)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::of(&[Keyword::Flying]),
        ability: None,
    };
    animate(g, me, me, &animation, Duration::Indefinite);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            YOUR_END_STEP,
            becomes_spirit_creature,
            "At the beginning of your end step, if you haven't cast a spell from your hand this turn and this enchantment isn't a creature, it becomes a 3/3 Spirit creature with flying in addition to its other types.",
        )
        .condition(no_spell_from_hand_and_not_a_creature),
        ability(cost("{2}{U}"), |g, item| effects::surveil(g, item, 1), "{2}{U}: Surveil 1."),
    ],
    ..oracle::EMERGENT_HAUNTING
};
