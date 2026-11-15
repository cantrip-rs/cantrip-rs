use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, exile_target_until_leaves, targets};
use cantrip_oracle as oracle;

/// "Exile target creature an opponent controls until this creature leaves the battlefield. If that creature had power 2 or
/// less, put a +1/+1 counter on this creature." The power it "had" is the one it had on the battlefield (CR 608.2h), so it
/// is read before the exile.
fn exile_until_leaves_then_counter_if_power_2_or_less(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = targets::object(g, item, 0).and_then(|id| g.chars(id).power);
    exile_target_until_leaves(g, item, 0);
    g.end_sentence();
    if power.is_some_and(|p| p <= 2) {
        effects::plus_one_counter_on_this(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, exile_until_leaves_then_counter_if_power_2_or_less, "When this creature enters, exile target creature an opponent controls until this creature leaves the battlefield. If that creature had power 2 or less, put a +1/+1 counter on this creature.")
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::DRIFTGLOOM_COYOTE
};
