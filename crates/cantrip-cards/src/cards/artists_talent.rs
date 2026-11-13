use cantrip_api::*;
use cantrip_mechanics::{class, cost_reduction, effects, replacements, triggers};
use cantrip_oracle as oracle;

/// Level 2: "Noncreature spells you cast cost {1} less to cast" (CR 601.2f, 716.2): only once the Class has reached level 2.
fn noncreature_spells_cost_one_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if class::level(q, source) < 2 { CostDelta::NONE } else { cost_reduction::noncreature_spells_you_cast(q, source, player, costed, 1) }
}

/// Level 3: "If a source you control would deal noncombat damage to an opponent or a permanent an opponent controls, it deals
/// that much damage plus 2 instead" (CR 614.1, 120.3), once the Class has reached level 3: the shared replacement for the
/// damage, and the level for the Class.
fn damage_to_an_opponent_or_their_permanent(q: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    class::level(q, asker.source) >= 3 && replacements::noncombat_damage_you_deal_to_an_opponent_or_their_permanent(q, asker, p)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::NONCREATURE_SPELL_YOU_CAST, |g, item| effects::may_discard_to_draw(g, item, 1), "Whenever you cast a noncreature spell, you may discard a card. If you do, draw a card."),
        class::level_two("{2}{R}", "{2}{R}: Level 2"),
        class::level_three("{2}{R}", "{2}{R}: Level 3"),
    ],
    cost_changes: &[noncreature_spells_cost_one_less],
    replacements: &[replacement(ReplacementKind::Damage, damage_to_an_opponent_or_their_permanent, replacements::plus::<2>, "deals that much damage plus 2 instead")],
    ..oracle::ARTISTS_TALENT
};
