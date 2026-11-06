use cantrip_api::*;
use cantrip_mechanics::triggers::AN_OPPONENT_DRAWS_A_CARD;
use cantrip_oracle as oracle;

/// "Whenever an opponent draws a card, this enchantment deals 1 damage to that player." (CR 608.2, 120.3).
fn enchantment_deals_1_damage_player(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let recipients_0 = item
        .subject
        .player()
        .into_iter()
        .map(Target::Player)
        .collect::<Vec<Target>>();
    let batch: Vec<(ObjectId, Target, u32)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, 1u32))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            AN_OPPONENT_DRAWS_A_CARD,
            enchantment_deals_1_damage_player,
            "Whenever an opponent draws a card, this enchantment deals 1 damage to that player.",
        ),
    ],
    ..oracle::UNDERWORLD_DREAMS
};
