use cantrip_api::*;
use cantrip_mechanics::players;
use cantrip_mechanics::targets::filters::nonland;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_oracle as oracle;

/// "Put a blight counter on it. Then for each blight counter on it, each opponent loses 4 life unless that player sacrifices
/// a nonland permanent of their choice or discards a card." The counter goes on it only while it is on the battlefield
/// (`add_counters` puts none on an object that has left, CR 400.7); the counters are counted on it, or as it last existed
/// there (CR 608.2h). The punishment, `players::each_opponent_loses_life_unless_sacrifices_or_discards`, once for each counter.
fn blight_and_punish(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Blight, 1);
    let counters = g.obj(item.object).counters(CounterKind::Blight).max(0);
    players::each_opponent_loses_life_unless_sacrifices_or_discards(g, item, &CARD, nonland, counters as u32, 4)
}

pub static CARD: CardDef = CardDef {
    // "As an additional cost to cast this spell, you may sacrifice any number of nonland permanents. This spell costs {1}
    // less to cast for each permanent sacrificed this way" (CR 601.2b, 601.2f, 118.7a): how many is announced, the total
    // cost determined with it, and which ones chosen as it is paid.
    additional_cost: Some(Cost { variable: Some(VariableCost::SacrificeAnyNumber { filter: nonland, less_each: 1 }), ..Cost::NONE }),
    abilities: &[triggered(THIS_ENTERS_OR_ATTACKS, blight_and_punish, "Whenever this creature enters or attacks, put a blight counter on it. Then for each blight counter on it, each opponent loses 4 life unless that player sacrifices a nonland permanent of their choice or discards a card.")],
    labels: &["Sacrifice a nonland permanent", "Discard a card", "Lose 4 life"],
    ..oracle::ROTTENMOUTH_VIPER
};
