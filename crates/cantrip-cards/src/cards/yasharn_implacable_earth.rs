use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_mechanics::select::{self, part, up_to};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// Whether `cards` can be "a basic Forest card and a basic Plains card", or some of that (CR 701.23b: either may be left
/// unfound): none, one that is either, or two that are one each. Forest and Plains are read through Yasharn's words (CR 612).
fn one_of_each(q: &dyn Query, source: ObjectId, cards: &[ObjectId]) -> bool {
    let words = q.words(source);
    let (forest, plains) = (words.subtype(Subtype::Forest), words.subtype(Subtype::Plains));
    match cards {
        [] => true,
        [a] => select::is_basic_of_type(q, *a, forest) || select::is_basic_of_type(q, *a, plains),
        [a, b] => select::is_basic_of_type(q, *a, forest) && select::is_basic_of_type(q, *b, plains) || select::is_basic_of_type(q, *a, plains) && select::is_basic_of_type(q, *b, forest),
        _ => false,
    }
}

/// The selection rule of "a basic Forest card and a basic Plains card": the cards chosen are some of one each, and a card
/// may be added while they still would be.
fn forest_and_plains(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let complete = one_of_each(q, choice.source, choice.selected);
    let addable = Bits::of(choice.candidates.len(), |i| {
        let card = choice.candidates[i];
        !choice.selected.contains(&card) && {
            let with: Vec<ObjectId> = choice.selected.iter().copied().chain([card]).collect();
            one_of_each(q, choice.source, &with)
        }
    });
    Verdict { complete, addable }
}

/// "When Yasharn enters, search your library for a basic Forest card and a basic Plains card, reveal those cards, put them
/// into your hand, then shuffle." Either may be left unfound (CR 701.23b; the ruling: "can find just one ... if you don't
/// wish to find both").
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = Selection::new(up_to(2)).with(part(&(forest_and_plains as SelectFn), 0, "one a basic Forest, one a basic Plains")).called("basic land");
    library::find_to_hand(g, item.controller, item.object, rule, true)?;
    Ok(())
}

/// "Players can't pay life or sacrifice nonland permanents to cast spells or activate abilities" (CR 101.2, 118.3, 601.2h,
/// 602.2b): every player, every spell and every activated ability, mana abilities included. A land may still be
/// sacrificed, and any amount of 0 life paid (CR 119.4b); what a resolving spell or ability has a player pay is untouched
/// (the rulings).
fn no_life_or_nonland_sacrifices(q: &dyn Query, _: &Asker, _: PlayerId, _: CostFor, payment: CostPayment) -> bool {
    match payment {
        CostPayment::Life => true,
        CostPayment::Sacrifice(permanent) => !q.chars(permanent).is(CardType::Land),
        CostPayment::BeamUp(_) => false,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        search,
        "When Yasharn enters, search your library for a basic Forest card and a basic Plains card, reveal those cards, put them into your hand, then shuffle.",
    )],
    cost_bans: &[no_life_or_nonland_sacrifices],
    ..oracle::YASHARN_IMPLACABLE_EARTH
};
