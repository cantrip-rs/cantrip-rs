use cantrip_api::*;
use cantrip_mechanics::{choose, choose_from_hand, players, select};
use cantrip_oracle as oracle;

/// "Each player loses half their life, then discards half the cards in their hand, then sacrifices half the permanents they
/// control of their choice. Round down each time." (CR 608.2c, 101.4, 107.1a) The three steps are done in order, and each
/// half is taken of what the player has when that step begins, so the life lost, the cards discarded and the permanents
/// sacrificed all shrink what the next step halves. A life total of zero or less has no half to lose (CR 107.1b).
///
/// In each step the players act together: they choose in turn order from the active player, each seeing what the players
/// before them chose, and every choice is carried out at once (CR 101.4), so "whenever a creature dies" sees them die
/// together. A player takes half the permanents they control, those that can't be sacrificed counted among them, and
/// sacrifices all of those that can when there are not that many (CR 701.21a, 101.2).
fn pox_plague(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().filter(|&p| players::reached(g, p)).collect();
    instruction(g, |g| {
        for &player in &players {
            let life = g.state().player(player).life;
            g.lose_life(player, life.max(0) / 2);
        }
    });
    let mut discarding = Vec::new();
    for &player in &players {
        let half = g.state().zone(Zone::hand(player)).len() as u32 / 2;
        discarding.extend(choose_from_hand(g, player, item.object, half)?);
    }
    g.discard_together(&discarding);
    let mut sacrificing = Vec::new();
    for &player in &players {
        let controlled = g.permanents_of(player);
        let half = controlled.len() / 2;
        let eligible: Vec<ObjectId> = controlled.into_iter().filter(|&id| g.can_sacrifice(player, id)).collect();
        let chosen = if eligible.len() <= half {
            eligible
        } else {
            choose::select(g, player, item.object, Purpose::Sacrifice, eligible, select::cards(select::exactly(half as u32)))?
        };
        sacrificing.extend(chosen.into_iter().map(|id| (player, id)));
    }
    g.sacrifice_each(&sacrificing);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], pox_plague), ..oracle::POX_PLAGUE };
