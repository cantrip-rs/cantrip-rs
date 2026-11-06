use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, cumulative_upkeep::Payment, effects, statics};
use cantrip_oracle as oracle;

/// All creatures present as the ability resolves get -1/-0 until end of turn (CR 611.2c).
fn shrink_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_all_creatures(g, item.object, -1, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(Payment::Mana(mana("{1}")), "Cumulative upkeep {1}"),
        ability(cost("{1}{G}"), shrink_creatures, "{1}{G}: All creatures get -1/-0 until end of turn."),
    ],
    statics: &[static_ability(Layer::PtModify, statics::all_creatures, statics::plus_pt_saturating::<-1, 0>, "All creatures get -1/-0.")],
    ..oracle::FYNDHORN_POLLEN
};
