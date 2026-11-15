use cantrip_api::*;
use cantrip_mechanics::{copy, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Activate only if Kitsa's power is 3 or greater" (CR 602.5b, 208.1): the power now, with the effects on it, judged as the ability
/// is activated and not again as it resolves.
fn this_power_at_least_3(q: &dyn Query, asker: &Asker) -> bool {
    q.chars(asker.source).power.unwrap_or(0) >= 3
}

/// Prowess; "{T}: Draw a card, then discard a card"; and "{2}, {T}: Copy target instant or sorcery spell you control. You may
/// choose new targets for the copy. Activate only if Kitsa's power is 3 or greater." (CR 707.10, 602.5b): the power is read as it
/// is activated (the ruling), and the copy is created on the stack, not cast.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::PROWESS,
        ability(TAP, effects::loot, "{T}: Draw a card, then discard a card."),
        ability(
            cost("{2}, {T}"),
            copy::copy_target_spell,
            "{2}, {T}: Copy target instant or sorcery spell you control. You may choose new targets for the copy. Activate only if Kitsa's power is 3 or greater.",
        )
        .targets(&[targets::INSTANT_OR_SORCERY_SPELL_YOU_CONTROL])
        .condition(this_power_at_least_3),
    ],
    ..oracle::KITSA_OTTERBALL_ELITE
};
