use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Return all creatures to their owners' hands. For each opponent who controlled a creature returned this way, you create two
/// 1/1 blue Faerie creature tokens with flying and 'This token can block only creatures with flying.'" Controllers are read before
/// the creatures leave (CR 400.7); only a creature that reached a hand counts (one exiled instead, CR 614.6, was not returned),
/// and an opponent counts once however many of their creatures came back. The tokens are created
/// together (CR 101.4) as one instruction after the return (CR 608.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<(ObjectId, PlayerId)> =
        g.state().battlefield().into_iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).map(|id| (id, g.controller(id))).collect();
    let ids: Vec<ObjectId> = creatures.iter().map(|&(id, _)| id).collect();
    g.return_to_hand(&ids);
    g.end_sentence();
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let affected = opponents.into_iter().filter(|&p| creatures.iter().any(|&(id, c)| c == p && g.obj(id).became.is_some_and(|new| g.obj(new).zone.is(ZoneKind::Hand)))).count() as u32;
    tokens::create(g, item.controller, &tokens::FAERIE_THAT_BLOCKS_ONLY_FLIERS, 2 * affected)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::FAERIE_SLUMBER_PARTY };

pub static EXTRA: &[&CardDef] = &[&tokens::FAERIE_THAT_BLOCKS_ONLY_FLIERS];
