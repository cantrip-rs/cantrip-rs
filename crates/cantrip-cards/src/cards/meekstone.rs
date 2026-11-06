use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Creatures with power 3 or greater don't untap during their controllers' untap steps." (CR 604.1).
fn creatures_power_3_greater_dont_rule(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
        && q.chars(id).power.is_some_and(|v| (i64::from(v)) >= 3)
}

pub static CARD: CardDef = CardDef {
    untap_bans: &[creatures_power_3_greater_dont_rule],
    ..oracle::MEEKSTONE
};
