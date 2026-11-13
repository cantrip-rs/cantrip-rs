use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// The chooser's creature or artifact with greatest mana value (CR 608.2d).
fn greatest(q: &dyn Query, choice: &Choice, artifact: u32) -> Verdict {
    let kind = if artifact == 0 { CardType::Creature } else { CardType::Artifact };
    let best = q.state().battlefield().iter().copied()
        .filter(|&id| q.controller(id) == choice.chooser && q.chars(id).is(kind))
        .map(|id| q.mana_value(id)).max();
    Verdict::each(choice, |id| q.controller(id) == choice.chooser
        && q.chars(id).is(kind) && Some(q.mana_value(id)) == best)
}

/// Each controller chooses in turn order; the exchange happens together (CR 101.4, 701.12a).
fn exchange_greatest(g: &mut dyn Game, item: &StackItem, other: PlayerId, artifact: bool) -> Result<(), Suspend> {
    let players: Vec<_> = g.state().apnap().filter(|&p| p == item.controller || p == other).collect();
    let mut chosen = Vec::new();
    for player in players {
        let kind = if artifact { CardType::Artifact } else { CardType::Creature };
        if !g.state().battlefield().iter().any(|&id| g.controller(id) == player && g.chars(id).is(kind)) {
            continue;
        }
        let rule = select::Selection::new(select::as_many_as_can(1))
            .called(if artifact { "artifact" } else { "creature" })
            .with(select::part(&(greatest as SelectFn), u32::from(artifact), "you control with greatest mana value"));
        let cards = g.state().battlefield().to_vec();
        let selection = choose::select(g, player, item.object, Purpose::Choose, cards, rule)?;
        if let Some(&id) = selection.first() { chosen.push(id); }
    }
    if chosen.len() == 2 {
        g.exchange_control(item.object, chosen[0], chosen[1]);
    }
    Ok(())
}

/// Exchange creatures first, then artifacts on the resulting battlefield (CR 608.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(other) = g.state().target(item, 0).player() else { return Ok(()) };
    exchange_greatest(g, item, other, false)?;
    g.end_sentence();
    exchange_greatest(g, item, other, true)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], effect),
    ..oracle::JUXTAPOSE
};
