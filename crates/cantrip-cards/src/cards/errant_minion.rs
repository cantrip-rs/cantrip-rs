use cantrip_api::*;
use cantrip_mechanics::{prevention, targets, triggers};
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "PayToPreventAuraDamage",
    "pay any amount of mana?",
    "This Aura then deals 2 damage to you, and 1 of that damage is prevented for each mana you paid",
    "Pay {cost}",
    "Don't pay",
);

/// "That player may pay any amount of mana. This Aura deals 2 damage to that player. Prevent X of that damage, where X is the
/// amount of mana that player paid this way." (CR 118.12, 608.2c, 615.7.) "That player" is the enchanted creature's controller
/// whose upkeep it is, fixed as the ability triggers (CR 603.3), whoever controls the creature or the Aura by now. Any amount of
/// mana is one mana, of any type, paid any number of times, the number chosen by the payer as they pay - a cost with no {X} in
/// it, so mana that may be spent only on costs that contain {X} can't pay it; declining pays none (a player who has left the
/// game pays nothing, CR 800.4f, and is dealt no damage, CR 800.4a). The damage is dealt by the Aura - as it last existed, if
/// it has left the battlefield (CR 113.7a) - and the prevention is a shield of X on that damage alone, ordered by that player
/// with any other effect on it (CR 616.1), preventing none of damage that can't be prevented (CR 615.12) and none of any later
/// damage.
fn pay_then_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = PlayerId(item.data[0] as u8);
    let paid = g.may_pay_times(player, item.object, item.object, mana("{1}"), ANY_NUMBER_OF_TIMES, &PAY)?;
    g.damage_preventing(item.object, Target::Player(player), 2, paid, item.ability.card, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
        pay_then_damage,
        "At the beginning of the upkeep of enchanted creature's controller, that player may pay any amount of mana. This Aura deals 2 damage to that player. Prevent X of that damage, where X is the amount of mana that player paid this way.",
    )],
    enchant: Some(targets::CREATURE),
    granted_preventions: &[GrantedPreventionDef { hint: "prevents X of that damage, X the mana paid", ..prevention::DAMAGE_DEALT_BY }
        .text("Prevent X of that damage, where X is the amount of mana that player paid this way.")],
    ..oracle::ERRANT_MINION
};
