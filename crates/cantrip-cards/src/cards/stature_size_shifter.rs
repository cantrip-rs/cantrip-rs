use cantrip_api::*;
use cantrip_mechanics::power_up;
use cantrip_oracle as oracle;

/// "Stature can't be blocked if her power is 1 or less" (CR 509.1b): of a possible block of Stature, her power as it is as
/// blockers are declared; a block that has been made stands if her power grows afterwards.
fn cant_block_stature_while_her_power_is_one_or_less(q: &dyn Query, source: ObjectId, _: ObjectId, attacker: ObjectId) -> bool {
    !(attacker == source && q.chars(source).power.is_some_and(|power| power <= 1))
}

/// "Put X +1/+1 counters on Stature." X is the one announced for the ability.
fn x_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, item.x as i16);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    block_rules: &[cant_block_stature_while_her_power_is_one_or_less],
    abilities: &[power_up::ability(
        cost("{X}{U}{U}"),
        x_counters,
        "Power-up \u{2014} {X}{U}{U}: Put X +1/+1 counters on Stature. (Activate each power-up ability only once. Reduce the cost by her mana cost if she entered this turn.)",
    )],
    ..oracle::STATURE_SIZE_SHIFTER
};
