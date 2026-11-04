use cantrip_api::*;
use cantrip_mechanics::{damage_each_target, effects, targets};
use cantrip_oracle as oracle;

/// "This creature deals 1 damage to target player or planeswalker." (CR 120.2, 608.2b).
fn deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_each_target(g, item, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP,
            deal_damage,
            "{T}: This creature deals 1 damage to target player or planeswalker.",
        ).targets(&[targets::PLAYER_OR_PLANESWALKER]),
        ability(
            cost("{1}{W}{W}").and(TAP),
            effects::return_first_target_to_hand,
            "{1}{W}{W}, {T}: Return target creature an opponent controls to its owner's hand.",
        ).targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
    ],
    ..oracle::WITCH_HUNTER
};
