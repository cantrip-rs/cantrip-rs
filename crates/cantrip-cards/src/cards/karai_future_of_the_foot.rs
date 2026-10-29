use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_mechanics::{effects, sneak, targets};
use cantrip_oracle as oracle;

/// "If Karai's sneak cost was paid this turn" (CR 702.190a): she was cast for it, and that cast is this turn's. The `Cast` record
/// is the one `was_cast_for_sneak` reads (the spell, or the permanent it became), here only asked whether it is of this turn.
fn sneak_cost_paid_this_turn(q: &dyn Query, karai: ObjectId) -> bool {
    sneak::was_cast_for_sneak(q, karai)
        && q.history().this_turn().iter().any(|r| matches!(&r.event, Event::Cast(c) if c.spell == karai || q.obj(c.spell).became == Some(karai)))
}

/// Returns the chosen creature card to its owner's hand, or to the battlefield if her sneak cost was paid this turn.
fn return_the_creature_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if sneak_cost_paid_this_turn(g, item.object) {
        effects::return_first_target_to_battlefield(g, item)
    } else {
        effects::return_first_target_to_hand(g, item)
    }
}

pub static CARD: CardDef = CardDef {
    ways: &[sneak::way(mana("{2}{W}{B}"), "Sneak {2}{W}{B}")],
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER,
        return_the_creature_card,
        "Whenever Karai deals combat damage to a player, return target creature card from your graveyard to your hand. If Karai's sneak cost was paid this turn, instead return it to the battlefield.",
    )
    .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::KARAI_FUTURE_OF_THE_FOOT
};
