use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// Choose a player who cast a sorcery this turn, then one of those spells and deal half its total damage (CR 608.2d).
fn backdraft(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<Target> = g.state().apnap()
        .filter(|&player| g.history().casts_by(player).any(|cast| cast.is(CardType::Sorcery)))
        .map(Target::Player).collect();
    let Some(Target::Player(player)) = choose::one_target(g, item.controller, item.object, players,
        "Choose a player who cast a sorcery spell this turn")? else { return Ok(()) };
    g.end_sentence();
    let spells: Vec<ObjectId> = g.history().casts_by(player)
        .filter(|cast| cast.is(CardType::Sorcery)).map(|cast| cast.spell).collect();
    let chosen = choose::select(g, item.controller, item.object, Purpose::Choose, spells,
        select::Selection::new(select::Count::Exactly(1)).called("sorcery spell cast this turn"))?;
    let Some(&spell) = chosen.first() else { return Ok(()) };
    let total: u128 = g.history().this_turn().iter().map(|r| match r.event {
        Event::Damage { source, amount, .. } if source == spell => amount.max(0) as u128,
        _ => 0,
    }).sum();
    g.damage(item.object, Target::Player(player), (total / 2).min(u32::MAX as u128) as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], backdraft),
    ..oracle::BACKDRAFT
};
