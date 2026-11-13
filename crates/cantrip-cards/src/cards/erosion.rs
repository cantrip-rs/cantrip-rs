use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST};
use cantrip_oracle as oracle;

/// Destroy that land unless its controller at the upkeep pays {1} or 1 life (CR 608.2, 701.8a, 118.1).
fn destroy_land_unless_player_pays(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = PlayerId(item.data[0] as u8);
    let paid = g.may_pay(player, item.object, item.object, cost("{1}"), &PAY_QUESTION)?
        || g.may_pay(player, item.object, item.object, life(1), &PAY_QUESTION)?;
    if !paid {
        g.destroy_all(&item.subject.object().into_iter().collect::<Vec<_>>());
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new("Pay", "pay {cost}?", "", "Pay {cost}", "Decline");

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
        destroy_land_unless_player_pays,
        "At the beginning of the upkeep of enchanted land's controller, destroy that land unless that player pays {1} or 1 life.",
    )],
    enchant: Some(targets::LAND),
    ..oracle::EROSION
};
