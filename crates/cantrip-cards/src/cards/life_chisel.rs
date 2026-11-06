use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::{targets, your_upkeep};

/// "Sacrifice a creature: You gain life equal to the sacrificed creature's toughness. Activate only during your upkeep." (CR 608.2h, 119.3).
fn gain_life_equal_sacrificed_creatures(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_0 = {
        let q: &dyn Query = g;
        q
            .cost_objects(item)
            .to_vec()
            .get(0)
            .copied()
            .map_or(0, |o: ObjectId| q.chars(o).toughness.unwrap_or(0))
            .max(0)
    };
    g.gain_life(item.controller, amount_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                sacrifice(1, targets::filters::creature),
                gain_life_equal_sacrificed_creatures,
                "Sacrifice a creature: You gain life equal to the sacrificed creature's toughness. Activate only during your upkeep.",
            )
            .condition(your_upkeep),
    ],
    ..oracle::LIFE_CHISEL
};
