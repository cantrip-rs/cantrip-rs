use cantrip_api::*;
use cantrip_mechanics::{replacements, triggers};
use cantrip_oracle as oracle;

/// "If a source you control would deal noncombat damage to a creature an opponent controls" (CR 614.1, 120.3): the shared
/// replacement for noncombat damage a source you control deals to an opponent or their permanent, narrowed to a creature
/// (a planeswalker or a player is dealt its damage as usual).
fn noncombat_damage_to_an_opponents_creature(q: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    let Proposal::Damage(d) = p else { return false };
    replacements::noncombat_damage_you_deal_to_an_opponent_or_their_permanent(q, asker, p)
        && d.target.object().is_some_and(|id| q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// "... put that many -1/-1 counters on that creature instead" (CR 614.1a, 614.6): the damage is not dealt, so it is replaced by
/// nothing (no damage dealt, no lifelink, no damage trigger) and `then` puts the counters. The counters are put by the
/// replacement effect's controller, as its `then` runs (CR 609.1). The effect is not a prevention
/// effect (damage that can't be prevented is replaced all the same).
fn damage_is_not_dealt(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount = 0;
    }
}

/// The counters put in place of the damage: as many -1/-1 counters as the damage was.
fn put_minus_one_counters(g: &mut dyn Game, _: &Asker, before: &Proposal, _: &Proposal) {
    if let Proposal::Damage(d) = before
        && let Some(creature) = d.target.object()
    {
        g.add_counters(creature, CounterKind::MinusOne, d.amount);
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::PROWESS],
    replacements: &[replacement(
        ReplacementKind::Damage,
        noncombat_damage_to_an_opponents_creature,
        damage_is_not_dealt,
        "puts that many -1/-1 counters on that creature instead",
    )
    .then(put_minus_one_counters)],
    ..oracle::SOUL_SCAR_MAGE
};
