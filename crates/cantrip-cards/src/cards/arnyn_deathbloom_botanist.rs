use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever a creature you control with power or toughness 1 or less dies" (CR 700.4, 603.10a): the creature as it last was on
/// the battlefield, with the characteristics and controller it had there, so Arnyn sees a creature that died with it, and a
/// negative power or toughness counts as 1 or less. Each such creature that dies triggers it once.
fn small_creature_you_control_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { old, from, to, .. } = event else { return None };
    if !from.is(ZoneKind::Battlefield) || !to.is(ZoneKind::Graveyard) || q.obj(*old).controller != me.controller {
        return None;
    }
    let chars = q.chars(*old);
    if chars.is(CardType::Creature) && (chars.power.is_some_and(|p| p <= 1) || chars.toughness.is_some_and(|t| t <= 1)) {
        return Fired::PLAIN;
    }
    None
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, small_creature_you_control_dies),
        |g, item| effects::target_player_loses_life_you_gain_life(g, item, 2),
        "Whenever a creature you control with power or toughness 1 or less dies, target opponent loses 2 life and you gain 2 life.",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::ARNYN_DEATHBLOOM_BOTANIST
};
