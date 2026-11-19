use cantrip_api::*;
use cantrip_mechanics::{discover, targets, triggers};
use cantrip_oracle as oracle;

fn in_one_graveyard(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let first = choice.selected.first().map(|&c| q.obj(c).zone);
    Verdict {
        complete: choice.selected.iter().all(|&c| Some(q.obj(c).zone) == first),
        addable: Bits::of(choice.candidates.len(), |i| first.is_none_or(|z| q.obj(choice.candidates[i]).zone == z)),
    }
}

fn largest_in_one_graveyard(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    choice
        .candidates
        .iter()
        .map(|&a| choice.candidates.iter().copied().filter(|&b| q.obj(b).zone == q.obj(a).zone).collect::<Vec<_>>())
        .max_by_key(Vec::len)
        .unwrap_or_default()
}

/// "In a single graveyard" (CR 404.1, 115.1c): every card chosen lies in the same player's graveyard.
const IN_A_SINGLE_GRAVEYARD: Part = select::set_part(&(in_one_graveyard as SelectFn), &(largest_in_one_graveyard as cantrip_api::select::LargestFn), 0, "in a single graveyard");

/// "Exile up to four target cards from a single graveyard." (CR 701.13): the targets still in their graveyard are exiled
/// together; one that became illegal is left alone (CR 608.2b).
fn exile_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.exile_all(&targets::legal_objects(g, item));
    Ok(())
}

const PAY_TO_DISCOVER: Question = Question::new("Pay", "pay {cost} to discover 4?", "", "Pay {cost}", "Decline");

/// "When this creature dies, you may pay {4}. If you do, discover 4." (CR 608.2d, 118.12, 701.57a)
fn may_pay_to_discover_4(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{4}"), &PAY_TO_DISCOVER)? {
        discover::discover(g, item.controller, 4)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(SACRIFICE_THIS, exile_them, "Sacrifice this creature: Exile up to four target cards from a single graveyard. Activate only as a sorcery.")
            .targets(&[targets::CARD_IN_A_GRAVEYARD.between(0, 4).together(IN_A_SINGLE_GRAVEYARD)])
            .sorcery_speed(),
        triggered(triggers::THIS_DIES, may_pay_to_discover_4, "When this creature dies, you may pay {4}. If you do, discover 4."),
    ],
    ..oracle::DIGSITE_CONSERVATOR
};
