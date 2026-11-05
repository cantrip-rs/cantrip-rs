use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

fn eligible(q: &dyn Query, card: ObjectId) -> bool {
    let c = q.chars(card);
    c.is(CardType::Artifact) || c.is(CardType::Creature) || c.is(CardType::Enchantment) || c.is(CardType::Land)
}

/// Each player may put an artifact, creature, enchantment, or land card from their hand onto the battlefield: each
/// chooses in turn order from the active player (CR 101.4), one not seeing another's choice; a player with nothing
/// that qualifies is not asked; then all the chosen cards enter at once, each under its chooser's control.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut entering = Vec::new();
    for player in g.state().apnap().collect::<Vec<_>>() {
        if !cantrip_mechanics::players::reached(g, player) {
            continue;
        }
        let hand = g.state().zone(Zone::hand(player)).to_vec();
        let can: Vec<ObjectId> = hand.iter().copied().filter(|&c| eligible(g, c)).collect();
        if let Some(&card) = choose::cards_from(g, player, item.object, Purpose::PutOntoBattlefield, hand, can, 1, true)?.first() {
            entering.push((card, player));
        }
    }
    g.put_all_onto_battlefield_by(&entering.into_iter().map(|(card, player)| (card, player, false, player)).collect::<Vec<_>>(), &[])?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::SHOW_AND_TELL };
