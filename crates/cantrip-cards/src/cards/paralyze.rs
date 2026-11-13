use cantrip_api::*;
use cantrip_mechanics::{attach::{attached_to_it, tap_enchanted}, targets, triggers};
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "Pay", "pay {cost} to untap the enchanted creature?", "", "Pay {cost}", "Don't pay",
);

/// The upkeep player may pay even if control or attachment has since changed. The Aura's current attachment, or its
/// last attachment if it left, identifies the creature to untap (CR 303.4m, 608.2h).
fn pay_to_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = PlayerId(item.data[0] as u8);
    if g.state().in_game(player) && g.may_pay(player, item.object, item.object, cost("{4}"), &PAY)? {
        g.end_sentence();
        if let Some(creature) = g.obj(item.object).attached_to {
            g.untap(creature);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[
        triggered(triggers::THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature."),
        triggered(
            triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
            pay_to_untap,
            "At the beginning of the upkeep of enchanted creature's controller, that player may pay {4}. If the player does, untap the creature.",
        ),
    ],
    untap_bans: &[attached_to_it],
    ..oracle::PARALYZE
};
