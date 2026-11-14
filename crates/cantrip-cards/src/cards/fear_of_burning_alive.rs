use cantrip_api::*;
use cantrip_mechanics::triggers::{SOURCE_YOU_CONTROL_DEALS_NONCOMBAT_DAMAGE_TO_AN_OPPONENT, THIS_ENTERS};
use cantrip_mechanics::{delirium, effects, targets};
use cantrip_oracle as oracle;

/// The intervening "if" (CR 603.4): delirium, checked as the ability triggers and again as it resolves.
fn has_delirium(q: &dyn Query, asker: &Asker) -> bool {
    delirium(q, asker.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| {
            effects::damage_each_opponent(g, item, 4);
            Ok(())
        }, "When this creature enters, it deals 4 damage to each opponent."),
        triggered(
            SOURCE_YOU_CONTROL_DEALS_NONCOMBAT_DAMAGE_TO_AN_OPPONENT,
            effects::this_deals_that_much_damage_to_target,
            "Delirium \u{2014} Whenever a source you control deals noncombat damage to an opponent, if there are four or more card types among cards in your graveyard, this creature deals that amount of damage to target creature that player controls.",
        )
        .targets(&[targets::CREATURE_THAT_PLAYER_CONTROLS])
        .condition(has_delirium)
        .damage_amount(),
    ],
    ..oracle::FEAR_OF_BURNING_ALIVE
};
