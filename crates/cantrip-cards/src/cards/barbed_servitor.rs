use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target opponent loses that much life." (CR 119.3, 120.2): the damage this creature was dealt at once, the trigger's matched
/// amount, even when it was lethal; nothing if the opponent is no longer a legal target as the ability resolves (CR 608.2b).
fn target_opponent_loses_life_equal_to_the_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(opponent) = targets::player(g, item, 0) {
        g.lose_life(opponent, item.matched_damage_amount() as i32);
    }
    Ok(())
}

/// "Suspect it" (CR 701.60a): this creature becomes suspected, with menace and "can't block" until it leaves the battlefield.
fn suspect_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.suspect(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, suspect_it, "When this creature enters, suspect it. (It has menace and can't block.)"),
        triggered(
            triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
            effects::draw_a_card_and_lose_1_life,
            "Whenever this creature deals combat damage to a player, you draw a card and you lose 1 life.",
        ),
        triggered(
            triggers::THIS_TAKES_DAMAGE,
            target_opponent_loses_life_equal_to_the_damage,
            "Whenever this creature is dealt damage, target opponent loses that much life.",
        )
        .targets(&[targets::OPPONENT])
        .damage_amount()
        .sums_per_subject(),
    ],
    ..oracle::BARBED_SERVITOR
};
