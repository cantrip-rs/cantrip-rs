use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::triggers::LANDFALL;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This enchantment becomes a Bear creature in addition to its other types and gains 'This creature's power and toughness are
/// each equal to the number of lands you control.' (This effect doesn't end.)" (CR 205.1b, 613.1d, 613.1f, 611.2a): the effects
/// are on the object that is the enchantment now, and only on it (CR 400.7), and last as long as the game does. The Bear type is
/// read through the words of this card's text (CR 612).
fn become_a_bear(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let bear = g.words(item.object).subtype(Subtype::Bear);
    let me = g.card_id(&CARD);
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddSubtype(bear), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(me, 0)), Duration::Indefinite);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            LANDFALL,
            effects::plus_one_counter_on_target,
            "Landfall \u{2014} Whenever a land you control enters, put a +1/+1 counter on target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        ability(
            cost("{5}{G}{G}"),
            become_a_bear,
            "{5}{G}{G}: This enchantment becomes a Bear creature in addition to its other types and gains \"This creature's power and toughness are each equal to the number of lands you control.\" (This effect doesn't end.)",
        ),
    ],
    delayed: &[granted_static(Layer::PtSet, lands::power_and_toughness_are_lands_you_control, "This creature's power and toughness are each equal to the number of lands you control.")],
    ..oracle::BEORNS_HOSPITALITY
};
