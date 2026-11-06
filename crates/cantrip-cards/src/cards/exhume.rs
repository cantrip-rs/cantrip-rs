use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// Each player puts a creature card from their graveyard onto the battlefield: each chooses in turn order (CR 101.4)
/// from their own graveyard (not a target), a player with no creature card there is not asked, and then all the
/// chosen cards enter at once (CR 110.2a), each under its chooser's control.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut entering = Vec::new();
    for player in g.state().apnap().collect::<Vec<_>>() {
        if !cantrip_mechanics::players::reached(g, player) {
            continue;
        }
        let graveyard = g.state().zone(Zone::graveyard(player)).to_vec();
        let can: Vec<ObjectId> = graveyard.iter().copied().filter(|&c| g.chars(c).is(CardType::Creature)).collect();
        if let Some(card) = choose::card_from(g, player, item.object, Purpose::PutOntoBattlefield, graveyard, can)? {
            entering.push((card, player));
        }
    }
    g.put_all_onto_battlefield_by(&entering.into_iter().map(|(card, player)| (card, player, false, player)).collect::<Vec<_>>(), &[])?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::EXHUME };
