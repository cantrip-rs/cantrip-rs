use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "Pay", "pay {cost} to avoid 2 damage and tapping that creature?", "", "Pay {cost}", "Don't pay",
);

/// The upkeep player and that creature are identified when the ability triggers (CR 603.3, 608.2h).
fn pay_or_damage_and_tap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = PlayerId(item.data[0] as u8);
    if !g.state().in_game(player) || !g.may_pay(player, item.object, item.object, cost("{3}"), &PAY)? {
        g.end_sentence();
        if g.state().in_game(player) {
            g.damage(item.object, Target::Player(player), 2);
        }
        if let Some(creature) = item.subject.object() {
            g.tap(creature);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
            pay_or_damage_and_tap,
            "At the beginning of the upkeep of enchanted creature's controller, that player may pay {3}. If they don't, this Aura deals 2 damage to that player and you tap that creature.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::MIND_WHIP
};
