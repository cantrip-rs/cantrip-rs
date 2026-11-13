use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return target creature card from your graveyard to your hand. If it's a Zombie card, draw a card." (CR 400.3, 608.2b): the
/// card is judged as it is in the hand - a Zombie there, by the words of this spell's own text (CR 612) - and a target that is no
/// longer legal means the spell does nothing at all (no draw).
fn return_and_draw_if_zombie(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let returned = g.return_to_hand(&[card]);
    g.end_sentence();
    if returned.iter().any(|&c| g.chars(c).has_subtype(g.words(item.object).subtype(Subtype::Zombie))) {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], return_and_draw_if_zombie),
    ..oracle::CEMETERY_RECRUITMENT
};
