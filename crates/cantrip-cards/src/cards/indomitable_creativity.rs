use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "Destroy X target artifacts and/or creatures. For each permanent destroyed this way, its controller reveals cards from
/// the top of their library until an artifact or creature card is revealed and exiles that card. Those players put the
/// exiled cards onto the battlefield, then shuffle."
///
/// X is announced with the spell and counts the targets (CR 601.2b-c); those still legal as it resolves are destroyed
/// together (CR 701.8a, 608.2b). A permanent counts as destroyed when it left the battlefield, even to a zone other than the
/// graveyard; one that stayed (indestructible, regenerated) does not, which is what `destroy_all` returns. Its controller
/// is the one it had as the spell resolved (CR 608.2h, 110.2a). Each reveal stops at the first card that is an artifact, a creature or both (CR 701.20a), and
/// takes the whole library when none is. The exiled cards still in exile (a replacement effect may have sent one elsewhere) enter together, each under the control of
/// the player who revealed it (CR 101.4), and every player who revealed shuffles, even when nothing was exiled.
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let targets: Vec<(ObjectId, PlayerId)> = targets::legal_objects(g, item).into_iter().map(|t| (t, g.controller(t))).collect();
    let ids: Vec<ObjectId> = targets.iter().map(|&(t, _)| t).collect();
    let destroyed = g.destroy_all(&ids);
    g.end_sentence();
    let mut exiled: Vec<(ObjectId, PlayerId)> = Vec::new();
    let mut revealers: Vec<PlayerId> = Vec::new();
    for (old, player) in targets {
        if !g.obj(old).became.is_some_and(|new| destroyed.contains(&new)) {
            continue;
        }
        if !revealers.contains(&player) {
            revealers.push(player);
        }
        let (found, _) = library::reveal_until(g, player, &|q, card| {
            let chars = q.chars(card);
            chars.is(CardType::Artifact) || chars.is(CardType::Creature)
        });
        if let Some(card) = found {
            exiled.push((g.exile(card), player));
        }
    }
    g.end_sentence();
    exiled.retain(|&(card, _)| library::is_still_exiled(g, card));
    g.put_all_onto_battlefield_by(&exiled.into_iter().map(|(card, player)| (card, player, false, player)).collect::<Vec<_>>(), &[])?;
    for player in revealers {
        g.shuffle(player);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_CREATURE.counted()], resolve),
    ..oracle::INDOMITABLE_CREATIVITY
};
