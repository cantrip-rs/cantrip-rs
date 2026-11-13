use cantrip_api::*;
use cantrip_mechanics::replacements::{damage_you_deal_to_an_opponent_or_their_permanent, plus};
use cantrip_mechanics::start_your_engines::has_max_speed;
use cantrip_mechanics::{effects, triggers::YOU_ATTACK};
use cantrip_oracle as oracle;

/// "Max speed - If a source you control would deal damage to an opponent or a permanent an opponent controls" (CR 702.178a, 614.1):
/// the damage a source you control would deal, while your speed is 4. Far Fortune's own attack trigger is a source it controls.
fn max_speed_damage_you_deal_to_an_opponent_or_their_permanent(q: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    has_max_speed(q.state(), asker.controller) && damage_you_deal_to_an_opponent_or_their_permanent(q, asker, p)
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[triggered(YOU_ATTACK, |g, item| Ok(effects::damage_each_opponent(g, item, 1)), "Whenever you attack, Far Fortune deals 1 damage to each opponent.")],
    replacements: &[replacement(
        ReplacementKind::Damage,
        max_speed_damage_you_deal_to_an_opponent_or_their_permanent,
        plus::<1>,
        "Max speed \u{2014} If a source you control would deal damage to an opponent or a permanent an opponent controls, it deals that much damage plus 1 instead.",
    )],
    ..oracle::FAR_FORTUNE_END_BOSS
};
