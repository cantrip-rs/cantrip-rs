use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Reveal the player's hand, then count the cards of the color named by this spell (CR 701.20a, 608.2h).
fn reveal_and_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    g.reveal(player, &hand);
    g.end_sentence();
    let color = g.words(item.object).color(Color::White);
    let amount = g.state().zone(Zone::hand(player)).iter().filter(|&&card| g.chars(card).colors.has(color)).count();
    g.damage(item.object, Target::Player(player),(amount) as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], reveal_and_damage),
    ..oracle::INQUISITION
};
