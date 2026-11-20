use cantrip_api::*;
use cantrip_mechanics::{copy, statics};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "An instant or sorcery spell that targets only a single creature you control" (CR 115.1): every target the spell has, as
/// it was announced (CR 601.2c), is the one creature, however many times it is chosen. A creature card in a graveyard is not a creature you control (CR 110.2). A spell with a player, or a second
/// object, as a target does not qualify, and neither does one with no targets.
fn instant_or_sorcery_targeting_only_one_creature_you_control(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let you = q.controller(source);
    let chars = q.chars(*spell);
    if *player != you || !(chars.is(CardType::Instant) || chars.is(CardType::Sorcery)) {
        return None;
    }
    let (_, first) = targets::spell_targeting_only_one_object(q, *spell)?;
    if !targets::permanent(q, first) || !q.chars(first).is(CardType::Creature) || q.controller(first) != you {
        return None;
    }
    Fired::about(Target::Object(*spell))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::SPELL_CAST, instant_or_sorcery_targeting_only_one_creature_you_control),
        copy::copy_the_cast_spell,
        "Whenever you cast an instant or sorcery spell that targets only a single creature you control, copy that spell. You may choose new targets for the copy.",
    )],
    opening_hand: Some(statics::BEGIN_ON_THE_BATTLEFIELD),
    ..oracle::LEYLINE_OF_RESONANCE
};
