use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Exile target nonland permanent. Transform Venat." (CR 701.13a, 701.27a): a target no longer legal leaves the ability no
/// legal target, so it does nothing at all and Venat does not transform (CR 608.2b).
fn heros_sundering(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile(permanent);
    g.end_sentence();
    effects::transform_self(g, item)
}

/// "Put a +1/+1 counter on another target creature you control. Until your next turn, it gains indestructible. If that
/// creature is legendary, draw a card." (CR 122.6, 611.2a): it is asked whether the creature is legendary as the ability
/// resolves; a target no longer legal leaves nothing done, and no card drawn (CR 608.2b).
fn blessing_of_light(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    g.end_sentence();
    g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(&[Keyword::Indestructible])), Duration::UntilNextTurnOf(item.controller));
    g.end_sentence();
    if g.chars(creature).supertypes.legendary {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        blessing_of_light,
        "Blessing of Light \u{2014} At the beginning of combat on your turn, put a +1/+1 counter on another target creature you control. Until your next turn, it gains indestructible. If that creature is legendary, draw a card.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::HYDAELYN_THE_MOTHERCRYSTAL
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::LEGENDARY_SPELL_YOU_CAST, |g, item| effects::draw(g, item, 1), "Whenever you cast a legendary spell, draw a card. This ability triggers only once each turn.")
            .triggers_each_turn(1),
        ability(cost("{7}, {T}"), heros_sundering, "Hero's Sundering \u{2014} {7}, {T}: Exile target nonland permanent. Transform Venat. Activate only as a sorcery.")
            .targets(&[targets::NONLAND_PERMANENT])
            .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::VENAT_HEART_OF_HYDAELYN
};
