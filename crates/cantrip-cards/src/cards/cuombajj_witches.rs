use cantrip_api::*;
use cantrip_mechanics::{damage_each_target, targets};
use cantrip_oracle as oracle;

/// "{T}: This creature deals 1 damage to any target and 1 damage to any target of an opponent's choice." (CR 608.2, 120.3).
fn creature_deals_1_damage_any(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_each_target(g, item, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                creature_deals_1_damage_any,
                "{T}: This creature deals 1 damage to any target and 1 damage to any target of an opponent's choice.",
            )
            .targets(&[targets::ANY, targets::ANY.of_an_opponents_choice()]),
    ],
    ..oracle::CUOMBAJJ_WITCHES
};
