use cantrip_api::*;
use cantrip_mechanics::{earthbend, effects, your_turn};
use cantrip_oracle as oracle;

/// "Whenever you attack a player with one or more creatures with power 4 or greater" (CR 508.1, 603.2c): a creature under the
/// controller's control declared as an attacker of a player rather than a planeswalker or battle, whose power is 4 or more as it is
/// declared (CR 508.1a, 603.2). Each player attacked is a batch of its own (`batch_by_data` on the defender), so the
/// ability triggers once for each player so attacked, however many such creatures attack them. A creature put onto the
/// battlefield attacking was never declared as an attacker (CR 508.4) and triggers nothing.
fn you_attack_a_player_with_a_creature_with_power_4_or_greater(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, defender: Target::Player(player), .. }
            if q.controller(*attacker) == me.controller && q.chars(*attacker).power.is_some_and(|power| power >= 4) =>
        {
            Some(Fired { subject: Target::Object(*attacker), data: [0, player.0 as u32] })
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ATTACKS, you_attack_a_player_with_a_creature_with_power_4_or_greater),
            |g, item| effects::draw(g, item, 1),
            "Whenever you attack a player with one or more creatures with power 4 or greater, draw a card.",
        )
        .once_per_batch()
        .batch_by_data(),
        ability(
            cost("{4}"),
            earthbend::effect::<4>,
            "Exhaust \u{2014} {4}: Earthbend 4. Activate only during your turn. (Target land you control becomes a 0/0 creature with haste that's still a land. Put four +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped. Activate each exhaust ability only once.)",
        )
        .targets(&[earthbend::TARGET])
        .exhaust()
        .condition(your_turn),
    ],
    delayed: &[earthbend::RETURN],
    ..oracle::BITTER_WORK
};
