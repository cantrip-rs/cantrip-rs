use cantrip_api::*;
use cantrip_mechanics::{behold, effects, flashback, targets, triggers};
use cantrip_oracle as oracle;

/// The ability the token copy has by the copy's exception (CR 707.9a). Not a card anyone plays: a definition for the
/// exception's ability to be referred to by. It is part of the copy's copiable values (CR 707.9a), so a copy of the token has
/// it too.
pub static EXCEPTION: CardDef = CardDef {
    name: "Kindle the Inner Flame (copy exception)",
    abilities: &[triggered(triggers::EACH_END_STEP, effects::sacrifice_self, "At the beginning of the end step, sacrifice this token.")],
    ..CardDef::BLANK
};

/// "Create a token that's a copy of target creature you control, except it has haste and [the exception's ability]" (CR 707.2,
/// 707.9a, 111.1): the creature as it is as the spell resolves, a token's original characteristics (CR 707.2), under the
/// spell's controller. Nothing if the creature is no longer a legal target (CR 608.2b).
fn copy_with_haste_and_sacrifice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let exception = AbilityRef { card: g.card_id(&EXCEPTION), index: 0 };
    let copy = g.copiable_values(creature).with_keyword(Keyword::Haste).with_ability(exception);
    g.create_token_copies_together(&[(copy, item.controller)])?;
    Ok(())
}

/// Flashback with an added cost (CR 702.34a, 118.8): {1}{R} plus beholding three Elementals (CR 701.4a), which exiles nothing.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], copy_with_haste_and_sacrifice),
    ways: &[flashback::way(cost("{1}{R}").and(behold::cost_of(3, behold::elemental, "Elemental")), "Flashback\u{2014}{1}{R}, Behold three Elementals")],
    ..oracle::KINDLE_THE_INNER_FLAME
};

pub static EXTRA: &[&CardDef] = &[&EXCEPTION];
