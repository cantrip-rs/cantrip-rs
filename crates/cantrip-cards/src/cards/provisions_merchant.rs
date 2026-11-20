use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_ENTERS};
use cantrip_mechanics::{effects, select, tokens};
use cantrip_oracle as oracle;

/// "You may sacrifice a Food. If you do, attacking creatures get +1/+1 and gain trample until end of turn." (CR 701.21a, 608.2c,
/// 611.2c): the sacrifice is chosen as the ability resolves, not paid as a cost, and the creatures attacking then, any player's,
/// are pumped only if a Food really was sacrificed. A Food is any permanent with the Food subtype; nothing is asked with none.
fn sacrifice_a_food_to_pump_attackers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, select::FOOD_CARD)?.is_empty() {
        effects::attacking_creatures_get_and_gain_trample(g, item.object, 1, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_food_token, "When this creature enters, create a Food token. (It's an artifact with \"{2}, {T}, Sacrifice this token: You gain 3 life.\")"),
        triggered(
            THIS_ATTACKS,
            sacrifice_a_food_to_pump_attackers,
            "Whenever this creature attacks, you may sacrifice a Food. If you do, attacking creatures get +1/+1 and gain trample until end of turn.",
        ),
    ],
    ..oracle::PROVISIONS_MERCHANT
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
