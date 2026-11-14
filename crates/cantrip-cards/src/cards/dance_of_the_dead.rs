use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets, triggers};
use cantrip_oracle as oracle;

/// That player may pay {1}{B}. If the player does, untap that creature.
fn pay_to_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    attach::upkeep_player_may_pay_to_untap_enchanted(g, item, cost("{1}{B}"))
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_CARD_IN_A_GRAVEYARD),
    becomes_aura_enchant: Some(targets::CREATURE_PUT_ONTO_THE_BATTLEFIELD_WITH_THIS),
    abilities: &[
        triggered(triggers::THIS_ENTERS, attach::put_enchanted_creature_card_onto_the_battlefield::<true>, "When this Aura enters, if it's on the battlefield, it loses \"enchant creature card in a graveyard\" and gains \"enchant creature put onto the battlefield with this Aura.\" Put enchanted creature card onto the battlefield tapped under your control and attach this Aura to it.")
            .condition(triggers::this_is_on_the_battlefield),
        triggered(triggers::THIS_LEAVES, attach::sacrifice_creature_put_onto_the_battlefield_with_this, "When this Aura leaves the battlefield, that creature's controller sacrifices it."),
        triggered(
            triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
            pay_to_untap,
            "At the beginning of the upkeep of enchanted creature's controller, that player may pay {1}{B}. If the player does, untap that creature.",
        ),
    ],
    statics: &[static_ability(Layer::PtModify, attach::attached_object, statics::plus_pt::<1, 1>, "Enchanted creature gets +1/+1.")],
    untap_bans: &[attach::attached_to_it],
    ..oracle::DANCE_OF_THE_DEAD
};
