use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_mechanics::{effects, hand, history, saddle, select};
use cantrip_oracle as oracle;

/// "A creature that saddled it this turn" (CR 702.171c): a creature tapped to pay the saddle cost of the Mount that asks, by id, so one
/// that has left the battlefield is read as it last existed (CR 608.2h).
fn saddled_it_this_turn(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |creature| q.history().saddled(choice.source).any(|saddler| saddler == creature))
}

const SADDLED_IT_THIS_TURN: select::Part = select::part(&(saddled_it_this_turn as SelectFn), 0, "that saddled it this turn");

/// "A creature that saddled it this turn", chosen as the ability resolves from the creatures its controller has.
const CREATURE_THAT_SADDLED_IT: Selection = Selection::of(select::up_to(1), &[select::CREATURE, SADDLED_IT_THIS_TURN]).called("creature that saddled it");

/// "You may sacrifice a creature that saddled it this turn. If you do, draw X cards, then put up to X land cards from your hand onto
/// the battlefield tapped, where X is the sacrificed creature's power" (CR 701.21a, 608.2c-d, 608.2h): the sacrifice is a choice made as
/// the ability resolves, not a cost, so nothing is asked when no saddler is left; X is the power the creature last had on the
/// battlefield, a negative one is no cards. The lands are chosen together after the draw, so a land just drawn can be put onto the
/// battlefield, and they are put, not played (CR 305.2).
fn sacrifice_a_saddler_to_draw_and_put_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let Some(&card) = effects::may_sacrifice(g, you, item.object, CREATURE_THAT_SADDLED_IT)?.objects().first() else { return Ok(()) };
    let x = history::power_of_sacrificed_permanent(g, card).max(0) as u32;
    g.draw(you, x);
    hand::may_put_up_to_onto_battlefield_tapped(g, you, item.object, select::land_cards(select::up_to(x)))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER,
            sacrifice_a_saddler_to_draw_and_put_lands,
            "Whenever The Gitrog deals combat damage to a player, you may sacrifice a creature that saddled it this turn. If you do, draw X cards, then put up to X land cards from your hand onto the battlefield tapped, where X is the sacrificed creature's power.",
        ),
        saddle::ability(1, "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::THE_GITROG_RAVENOUS_RIDE
};
