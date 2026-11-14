use cantrip_api::*;
use cantrip_mechanics::effects::{self, this_gains_indestructible_then_tap};
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Exile up to one target card from a graveyard and put a +1/+1 counter on this creature": with no target chosen the counter
/// still goes on, but with a target chosen and gone the ability does not resolve and none of it happens (CR 608.2b; the ruling of
/// 2024-11-08); the counter goes only on the creature still on the battlefield (CR 400.7).
fn exile_a_card_then_plus_one_counter_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::exile_first_target(g, item)?;
    g.end_sentence();
    effects::plus_one_counter_on_this(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_BECOMES_TAPPED,
            exile_a_card_then_plus_one_counter_on_this,
            "Whenever this creature becomes tapped, exile up to one target card from a graveyard and put a +1/+1 counter on this creature.",
        )
        .targets(&[targets::CARD_IN_A_GRAVEYARD.optional()]),
        ability(
            sacrifice(1, targets::filters::another_creature),
            this_gains_indestructible_then_tap,
            "Sacrifice another creature: This creature gains indestructible until end of turn. Tap it.",
        ),
    ],
    ..oracle::IMMERSTURM_PREDATOR
};
