use cantrip_api::select::{Bits, Choice, LargestFn, SelectFn, Verdict, set_part};
use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Share a creature type" (CR 205.3m, 205.2): some one creature type is a type of every card chosen, each read where it is
/// (a changeling has them all, CR 702.73a). A single card shares a type with itself, any of the cards of a set that shares
/// one share it too, and with nothing chosen yet every type is shared, so only a creature type of its own is wanted of a card.
fn share_a_creature_type(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let all_have = |t: Subtype| choice.selected.iter().all(|&c| q.chars(c).subtypes.has(t));
    let complete = Subtype::CREATURE_TYPES.iter().any(|&t| all_have(t));
    let addable = Bits::of(choice.candidates.len(), |i| {
        let candidate = q.chars(choice.candidates[i]);
        Subtype::CREATURE_TYPES.iter().any(|&t| all_have(t) && candidate.subtypes.has(t))
    });
    Verdict { complete, addable }
}

/// A largest set that shares a creature type: the candidates of the creature type the most of them have.
fn largest_sharing_a_creature_type(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    let with = |t: Subtype| choice.candidates.iter().copied().filter(|&c| q.chars(c).subtypes.has(t)).collect::<Vec<_>>();
    Subtype::CREATURE_TYPES.iter().map(|&t| with(t)).max_by_key(Vec::len).unwrap_or_default()
}

const SHARE_A_CREATURE_TYPE: Part =
    set_part(&(share_a_creature_type as SelectFn), &(largest_sharing_a_creature_type as LargestFn), 0, "that share a creature type");

/// "Return two target creature cards that share a creature type from your graveyard to your hand": both move together
/// (CR 400.3). The pair is judged again as it resolves (CR 608.2b), and one that no longer shares a type, or has a card gone
/// from the graveyard, returns neither (`TargetSpec::together`).
fn return_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let (Some(first), Some(second)) = (targets::object(g, item, 0), targets::object(g, item, 1)) {
        g.return_to_hand(&[first, second]);
    }
    Ok(())
}

/// "Choose one" (CR 700.2).
static MODAL: ModalDef = modes(&[
    mode(effects::return_first_target_to_hand, "Return target creature card from your graveyard to your hand.")
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    mode(return_two, "Return two target creature cards that share a creature type from your graveyard to your hand.")
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(2, 2).together(SHARE_A_CREATURE_TYPE)]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::UNBURY };
