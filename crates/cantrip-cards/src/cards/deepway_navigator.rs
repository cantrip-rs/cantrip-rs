use cantrip_api::*;
use cantrip_mechanics::statics::{plus_pt, your_of_subtype};
use cantrip_mechanics::{lands, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

const MERFOLK: u16 = lands::sub(Subtype::Merfolk);

/// "Untap each other Merfolk you control."
fn untap_other_merfolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let merfolk = g.words(item.object).subtype(Subtype::Merfolk);
    let others: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).has_subtype(merfolk)).collect();
    for id in others {
        g.untap(id);
    }
    Ok(())
}

/// "As long as you attacked with three or more Merfolk this turn" (CR 508.1, 611.3a): the creatures this ability's controller
/// declared as attackers this turn, each once however many combats it attacked in, that were Merfolk as they were declared
/// (whatever they are since, wherever they have gone), asked of the event history each time the layers apply.
fn attacked_with_three_merfolk(view: &dyn LayerQuery, source: ObjectId) -> bool {
    let merfolk = view.words(source).subtype(Subtype::Merfolk);
    let mut seen: Vec<ObjectId> = Vec::new();
    for attack in view.history().attacked_with(view.state().obj(source).controller) {
        let declared_merfolk = view.state().last_known.get(attack.known as usize).is_some_and(|row| row.id == attack.attacker && row.subtypes.has(merfolk));
        if declared_merfolk && !seen.contains(&attack.attacker) {
            seen.push(attack.attacker);
        }
    }
    seen.len() >= 3
}

/// "Merfolk you control", only while the controller attacked with three or more Merfolk this turn.
fn merfolk_while_attacked_with_three(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    your_of_subtype::<MERFOLK>(view, source, affected, so_far) && attacked_with_three_merfolk(view, source)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, untap_other_merfolk, "When this creature enters, untap each other Merfolk you control.")],
    statics: &[static_ability(
        Layer::PtModify,
        merfolk_while_attacked_with_three,
        plus_pt::<1, 0>,
        "As long as you attacked with three or more Merfolk this turn, Merfolk you control get +1/+0.",
    )],
    ..oracle::DEEPWAY_NAVIGATOR
};
