use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

const EXILE_AND_COLLECT: Question =
    Question::new("ExileAndCollectEvidence", "exile {subject} and collect evidence 4 to return it to the battlefield?", "", "Exile it and collect evidence", "Leave it");

/// "When this creature dies, you may exile it and collect evidence 4. If you do, return this card to the battlefield tapped"
/// (CR 603.10a, 701.59a, 608.2c, 400.7): the card the Phoenix became in the graveyard, asked only while it is still there and the
/// graveyard holds cards worth 4 or more besides it (CR 701.59b), which exiling the Phoenix itself can't supply since it is exiled
/// as part of the same payment. Both are paid together, so the Phoenix is never exiled without the evidence; paid, the card in exile
/// is put onto the battlefield tapped under its owner's control (a new object, CR 400.7).
fn exile_it_and_collect_evidence_to_return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = g.found_all_after_move(item.object).first().copied() else { return Ok(()) };
    if !g.may_pay(item.controller, card, card, Cost { collect_evidence: 4, ..EXILE_THIS }, &EXILE_AND_COLLECT)? {
        return Ok(());
    }
    g.end_sentence();
    let exiled: Vec<(ObjectId, PlayerId, bool)> = g.found_all_after_move(card).into_iter().filter(|&o| g.obj(o).zone.is(ZoneKind::Exile)).map(|o| (o, g.obj(o).owner, true)).collect();
    g.put_all_onto_battlefield_as(&exiled)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_DIES,
        exile_it_and_collect_evidence_to_return_it,
        "When this creature dies, you may exile it and collect evidence 4. If you do, return this card to the battlefield tapped.",
    )],
    ..oracle::LAMPLIGHT_PHOENIX
};
