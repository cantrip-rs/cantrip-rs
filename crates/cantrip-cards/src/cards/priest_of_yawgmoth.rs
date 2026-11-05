use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{T}, Sacrifice an artifact: Add an amount of {B} equal to the sacrificed artifact's mana value." (CR 605.1a, 106.3).
fn add_amount_b_equal_sacrificed_mana(
    g: &mut dyn Game,
    asker: &Asker,
    _: Option<Color>,
) -> Result<(), Suspend> {
    let mana_0_0 = {
        let q: &dyn Query = g;
        q
            .mana_ability_cost_objects(0, CostPart::Permanents)
            .get(0)
            .copied()
            .map_or(0, |o: ObjectId| i64::from(q.mana_value(o))) as u32
    };
    g.add_mana(asker.controller, Mana::B, mana_0_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP.and(sacrifice(1, targets::filters::artifact)),
            add_by(add_amount_b_equal_sacrificed_mana),
            "{T}, Sacrifice an artifact: Add an amount of {B} equal to the sacrificed artifact's mana value.",
        ),
    ],
    ..oracle::PRIEST_OF_YAWGMOTH
};
