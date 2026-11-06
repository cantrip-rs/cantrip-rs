use cantrip_api::*;
use cantrip_mechanics::{goyf, library, triggers, effects};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Mill", "mill that many cards?", "If you do, you may put a creature card from among them into your hand", "Mill", "Don't mill");

/// You may mill that many cards; if you do, you may put a creature card from among them into your hand (CR 701.17c).
fn mill_and_take(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let available = g.state().zone(Zone::library(item.controller)).len();
    let count = item.matched_damage_amount().min(available.min(u32::MAX as usize) as u32);
    library::mill_then_may_take_into_hand(g, item.controller, item.object, count, "creature", |c| c.is(CardType::Creature))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT, mill_and_take, "Whenever this creature deals combat damage to a player, you may mill that many cards. If you do, you may put a creature card from among them into your hand.").damage_amount()],
    statics: &[goyf::SIZE],
    ..oracle::BARROWGOYF
};
