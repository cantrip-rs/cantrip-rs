use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_IF_CAST;
use cantrip_oracle as oracle;

/// "When this creature enters, if you cast it, each player sacrifices all other creatures they control. Then each player
/// returns all creature cards from their graveyard that weren't put there this way to the battlefield." Every creature but
/// this one is sacrificed at once (CR 701.21, 101.4), whoever controls it; those the sacrifice put into a graveyard are the
/// new objects it returns and are left there. Then every player's other creature cards enter at once (CR 110.2a), each
/// under the control of the player whose graveyard it was in.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others: Vec<(PlayerId, ObjectId)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| id != item.object && g.chars(id).is(CardType::Creature))
        .map(|id| (g.controller(id), id))
        .collect();
    let sacrificed = g.sacrifice_each(&others);
    let mut returning = Vec::new();
    for player in g.state().apnap().collect::<Vec<_>>() {
        for &card in g.state().zone(Zone::graveyard(player)) {
            if !sacrificed.contains(&card) && g.chars(card).is(CardType::Creature) {
                returning.push((card, player));
            }
        }
    }
    g.put_all_onto_battlefield_by(&returning.into_iter().map(|(card, player)| (card, player, false, player)).collect::<Vec<_>>(), &[])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS_IF_CAST, effect, "When this creature enters, if you cast it, each player sacrifices all other creatures they control. Then each player returns all creature cards from their graveyard that weren't put there this way to the battlefield.")],
    ..oracle::BRINGER_OF_THE_LAST_GIFT
};
