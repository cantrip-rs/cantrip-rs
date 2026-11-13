use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "If it wasn't a Demon" (CR 603.4, 603.10a): judged on the permanent as it last was on the battlefield, its subtypes read through
/// the words the Vessel last had (CR 612). Once it returns a Demon, it is one when it dies again and the ability does not return it.
fn wasnt_a_demon(q: &dyn Query, asker: &Asker) -> bool {
    let demon = q.words(asker.source).subtype(Subtype::Demon);
    !q.chars(asker.source).has_subtype(demon)
}

/// "It's a Demon in addition to its other types" (CR 614.12, 205.1b): it enters as that, and stays one for as long as it is the
/// object that returned (CR 611.2a).
static AS_A_DEMON: EntersAs = EntersAs { subtypes: &[Subtype::Demon], ..EntersAs::NONE };

/// "Return it to the battlefield under its owner's control with two +1/+1 counters on it" (CR 400.7, 122.6, 730.3c): the card the
/// permanent became in the graveyard, if it is still there, each card of a merged permanent together under its owner.
fn return_as_a_demon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<(ObjectId, PlayerId)> = g.found_all_after_move(item.object).into_iter().map(|c| (c, g.obj(c).owner)).collect();
    g.put_all_onto_battlefield_becoming(&cards, item, &AS_A_DEMON, &[(CounterKind::PlusOne, 2)])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_DIES,
        return_as_a_demon,
        "When this creature dies, if it wasn't a Demon, return it to the battlefield under its owner's control with two +1/+1 counters on it. It's a Demon in addition to its other types.",
    )
    .condition(wasnt_a_demon)],
    ..oracle::INFERNAL_VESSEL
};
