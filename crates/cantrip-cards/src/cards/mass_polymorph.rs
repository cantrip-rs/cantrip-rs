use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

/// "Exile all creatures you control, then reveal cards from the top of your library until you reveal that many creature
/// cards. Put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into
/// your library" (CR 701.13, 701.20, 701.24). The number is how many creatures were exiled, tokens among them, and the
/// creature cards it finds enter at the same time (CR 110.2a, 603.6). Cards that are not creatures never leave the
/// library, so the shuffle at the end is the library's own (CR 701.24a); with nothing revealed it shuffles nothing, the
/// same empty shuffle the too-few-creature-cards ruling describes (2010-08-15), and with nothing exiled nothing is
/// revealed.
fn polymorph(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let exiled: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.controller(id) == you && g.chars(id).is(CardType::Creature))
        .collect();
    let wanted = exiled.len();
    g.exile_all(&exiled);
    let (found, _) = library::reveal_until(g, you, wanted, &|q, card| q.chars(card).is(CardType::Creature));
    let entering: Vec<(ObjectId, PlayerId)> = found.into_iter().map(|card| (card, you)).collect();
    g.put_all_onto_battlefield(&entering)?;
    g.shuffle(you);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], polymorph),
    ..oracle::MASS_POLYMORPH
};
