use cantrip_api::*;
use cantrip_mechanics::{statics::other_creatures_you_control, targets};
use cantrip_oracle as oracle;

/// "Each other creature you control has hexproof from each of its colors" (CR 702.11d, g): the colors are those the
/// creature has once layer 5 is done, so the ability layer reads them as they are (CR 613.1).
fn hexproof_from_own_colors(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.hexproof_from = chars.hexproof_from.and(HexproofFrom(chars.colors.0 as u32));
}

/// "Target creature you control becomes all colors until end of turn" (CR 105.2, 611.2a): it replaces its colors.
fn become_all_colors(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let words = g.words(item.object);
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        g.add_effect(item.object, creature, EffectKind::SetColors(words.colors(ColorSet(0x1f))), Duration::EndOfTurn);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        other_creatures_you_control,
        hexproof_from_own_colors,
        "Each other creature you control has hexproof from each of its colors.",
    )],
    abilities: &[ability(TAP, become_all_colors, "{T}: Target creature you control becomes all colors until end of turn.")
        .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::TAM_MINDFUL_FIRST_YEAR
};
