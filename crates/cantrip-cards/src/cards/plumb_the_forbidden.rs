use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::triggers::CAST_THIS_SPELL;
use cantrip_oracle as oracle;

/// "When you do" (CR 603.12): the cost of sacrificing creatures was paid, so the spell was "kicked" as many times as
/// creatures were sacrificed for it.
fn sacrificed_creatures(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).paid > 0
}

/// Copy this spell for each creature sacrificed this way (CR 707.10).
fn copy_for_each(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for _ in 0..g.obj(item.object).paid {
        g.copy_spell(item.object, item.controller, false)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::draw_a_card_and_lose_1_life),
    optional_cost: Some(OptionalCost::new(Cost { variable: Some(VariableCost::SacrificeOneOrMore(filters::creature)), ..Cost::NONE })),
    labels: &["Cast without sacrificing", "Sacrifice one or more creatures"],
    abilities: &[triggered(CAST_THIS_SPELL, copy_for_each, "When you do, copy this spell for each creature sacrificed this way.")
        .condition(sacrificed_creatures)],
    ..oracle::PLUMB_THE_FORBIDDEN
};
