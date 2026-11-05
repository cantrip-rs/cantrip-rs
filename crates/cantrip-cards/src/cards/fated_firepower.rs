use cantrip_api::*;
use cantrip_mechanics::{effects, replacements};
use cantrip_oracle as oracle;

/// "... it deals that much damage plus an amount of damage equal to the number of fire counters on this enchantment instead"
/// (CR 614.1, 120.3, 122.1): the counters it has as the damage is about to be dealt. The damage stays that of its source
/// (CR 614.5, the rulings), so a second Fated Firepower adds its counters once more.
fn plus_the_fire_counters(q: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount = d.amount.checked_add(i64::from(q.obj(asker.source).counters(CounterKind::Fire))).expect("Fated Firepower damage exceeds i64 semantic domain");
    }
}

/// Flash is a core keyword.
pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::Fire as usize }>),
    replacements: &[replacement(
        ReplacementKind::Damage,
        replacements::damage_you_deal_to_an_opponent_or_their_permanent,
        plus_the_fire_counters,
        "it deals that much damage plus an amount of damage equal to the number of fire counters on this enchantment instead",
    )],
    ..oracle::FATED_FIREPOWER
};
