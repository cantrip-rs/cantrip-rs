use cantrip_api::*;
use cantrip_mechanics::{targets, damage_divided};
use cantrip_oracle as oracle;

/// The total to divide as the spell is cast (CR 601.2d).
fn meteor_shower_deals_x_plus_divided_total(_: &dyn Query, _: &Asker, x: u32) -> u32 {
    x.saturating_add(1)
}

/// "Meteor Shower deals X plus 1 damage divided as you choose among any number of targets." (CR 608.2).
fn meteor_shower_deals_x_plus(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    damage_divided(g, item);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::ANY.between(0, TargetSpec::ANY_NUMBER)],
        meteor_shower_deals_x_plus,
    ),
    divide: Some(Divide {
        total: TargetSpec::COUNTED,
        reads: Some(meteor_shower_deals_x_plus_divided_total),
    }),
    ..oracle::METEOR_SHOWER
};
