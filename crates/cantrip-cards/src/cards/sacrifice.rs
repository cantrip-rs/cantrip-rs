use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "As an additional cost to cast this spell, sacrifice a creature. Add an amount of {B} equal to the sacrificed creature's mana value." (CR 608.2, 106.3).
fn as_additional_cost_cast_spell_2(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mana_2_0 = {
        let q: &dyn Query = g;
        q
            .history()
            .spell_cost_objects(
                item.object,
                CostOrigin::PrintedAdditional,
                0,
                CostPart::Permanents,
            )
            .collect::<Vec<ObjectId>>()
            .get(0)
            .copied()
            .map_or(0, |o: ObjectId| i64::from(q.mana_value(o))) as u32
    };
    g.add_mana(item.controller, Mana::B, mana_2_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], as_additional_cost_cast_spell_2),
    additional_cost: Some(sacrifice(1, targets::filters::creature)),
    ..oracle::SACRIFICE
};
