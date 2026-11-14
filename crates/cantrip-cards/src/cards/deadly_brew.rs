use cantrip_api::*;
use cantrip_mechanics::{choose, each_player_sacrifices, select};
use cantrip_oracle as oracle;

/// "Each player sacrifices a creature or planeswalker of their choice. If you sacrificed a permanent this way, you may return another
/// permanent card from your graveyard to your hand." Every player chooses in turn order and the permanents are sacrificed together (CR 101.4,
/// 701.21a). The card is chosen as the spell resolves, after the sacrifice (the ruling of 2021-04-16), from the caster's graveyard only,
/// and "another" leaves out every card the sacrificed permanent became (CR 400.7: a token became none, a merged permanent several).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let sacrificed = each_player_sacrifices(g, item.object, 1, |q, id| {
        let chars = q.chars(id);
        chars.is(CardType::Creature) || chars.is(CardType::Planeswalker)
    })?;
    g.end_sentence();
    let yours = sacrificed.into_iter().find(|&(who, _)| who == you).map(|(_, id)| id).filter(|&id| g.obj(id).became.is_some());
    let Some(permanent) = yours else { return Ok(()) };
    let itself = g.found_all_after_move(permanent);
    let cards: Vec<ObjectId> = g
        .state()
        .zone(Zone::graveyard(you))
        .iter()
        .copied()
        .filter(|card| !itself.contains(card))
        .collect();
    let chosen = choose::select(g, you, item.object, Purpose::ReturnToHand, cards, select::Selection::of(select::up_to(1), &[select::PERMANENT_CARD]).called("permanent"))?;
    g.return_to_hand(&chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::DEADLY_BREW };
