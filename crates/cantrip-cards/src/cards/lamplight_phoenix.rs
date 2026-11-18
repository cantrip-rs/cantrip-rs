use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

const EXILE_AND_COLLECT: Question =
    Question::new("ExileAndCollectEvidence", "exile {subject} and collect evidence 4 to return it to the battlefield?", "", "Exile it and collect evidence", "Leave it");

/// "When this creature dies, you may exile it and collect evidence 4. If you do, return this card to the battlefield tapped"
/// (CR 603.10a, 701.59a, 608.2c, 400.7): "it" is every card the creature became in the graveyard - each card of a merged
/// permanent (CR 730.3c) - asked only while they are all still there and the graveyard holds cards worth 4 or more besides
/// them (CR 701.59b), since none of them can be exiled twice. They and the evidence are exiled as one payment, so the
/// Phoenix is never exiled without the evidence; paid, each card in exile is put onto the battlefield tapped, a new object
/// (CR 400.7), under the control of the ability's controller, the player the effect has put it there (CR 110.2a).
fn exile_it_and_collect_evidence_to_return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let found = g.found_all_after_move(item.object);
    let Some((&card, with)) = found.split_first() else { return Ok(()) };
    if !g.may_pay_exiling(item.controller, card, with, Cost { collect_evidence: 4, ..EXILE_THIS }, &EXILE_AND_COLLECT)? {
        return Ok(());
    }
    g.end_sentence();
    let exiled: Vec<(ObjectId, PlayerId, bool)> = found
        .iter()
        .flat_map(|&o| g.found_all_after_move(o))
        .filter(|&o| g.obj(o).zone.is(ZoneKind::Exile))
        .map(|o| (o, item.controller, true))
        .collect();
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
