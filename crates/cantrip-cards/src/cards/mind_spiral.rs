use cantrip_api::*;
use cantrip_mechanics::{effects, gift, targets, tokens};
use cantrip_oracle as oracle;

/// "Target player draws three cards. If the gift was promised, tap target creature an opponent controls and put a stun counter
/// on it." (CR 702.174, 701.26a, 122.1d): the second target exists only when the gift was promised (it is announced before the
/// targets, CR 702.174m), so a creature target means it was. An already tapped creature still gets the counter.
fn draw_three_and_if_promised_tap_and_stun_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_draws(g, item, 0, 3);
    let Some(creature) = targets::object(g, item, 1) else { return Ok(()) };
    g.tap(creature);
    g.add_counters(creature, CounterKind::Stun, 1);
    Ok(())
}

/// "Gift a tapped Fish" (CR 702.174f): the Fish is the card's own token.
pub static EXTRA: &[&CardDef] = &[&tokens::FISH];

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::PLAYER, targets::CREATURE_AN_OPPONENT_CONTROLS.between(0, 0).if_promised(1, 1)],
        draw_three_and_if_promised_tap_and_stun_a_creature,
    ),
    gift: Some(gift::TAPPED_FISH),
    ..oracle::MIND_SPIRAL
};
