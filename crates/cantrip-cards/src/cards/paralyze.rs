use cantrip_api::*;
use cantrip_mechanics::{attach::{self, attached_to_it, tap_enchanted}, targets, triggers};
use cantrip_oracle as oracle;

/// That player may pay {4}. If the player does, untap the creature.
fn pay_to_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    attach::upkeep_player_may_pay_to_untap_enchanted(g, item, cost("{4}"))
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
