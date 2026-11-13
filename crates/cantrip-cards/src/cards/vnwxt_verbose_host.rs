use cantrip_api::*;
use cantrip_mechanics::replacements::{self, DRAW_TWO_INSTEAD};
use cantrip_mechanics::start_your_engines::has_max_speed;
use cantrip_oracle as oracle;

/// "As long as your speed is 4" (CR 702.178a) it replaces each of its controller's draws: "your" is the controller of the
/// permanent whose effect it is (CR 109.5), so it is judged by the speed they have as the draw would be made (CR 121.2).
fn you_would_draw_a_card_at_max_speed(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    replacements::you_would_draw_a_card(q, asker, proposal) && has_max_speed(q.state(), asker.controller)
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    // "Max speed — If you would draw a card, draw two cards instead." (CR 121.6, 614.1a): Thought Reflection's replacement, which
    // functions only at max speed. A permanent that has lost all its abilities has no replacement effects (CR 613.1f).
    replacements: &[ReplacementDef { applies: you_would_draw_a_card_at_max_speed, hint: "at max speed, draws two cards instead", ..DRAW_TWO_INSTEAD }],
    play_rules: &[PlayRule::MaximumHandSize(None)],
    ..oracle::VNWXT_VERBOSE_HOST
};
