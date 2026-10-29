use cantrip_api::select::{Bits, Choice, LargestFn, SelectFn, Verdict, set_part};
use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Share a creature type" (CR 205.3m, 205.2): some one creature type is a type of every card chosen, each read where it is
/// (a changeling has them all, CR 702.73a). A single card shares a type with itself, any of the cards of a set that shares
/// one share it too, and with nothing chosen yet every type is shared, so only a creature type of its own is wanted of a card.
fn share_a_creature_type(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let selected: Vec<SubtypeSet> = choice.selected.iter().map(|&c| q.chars(c).subtypes).collect();
    let complete = shares_a_creature_type(&selected);
    let addable = Bits::of(choice.candidates.len(), |i| may_join(&selected, q.chars(choice.candidates[i]).subtypes));
    Verdict { complete, addable }
}

/// Some one creature type is among `types` of every card (none chosen share every type).
fn shares_a_creature_type(selected: &[SubtypeSet]) -> bool {
    Subtype::CREATURE_TYPES.iter().any(|&t| selected.iter().all(|s| s.has(t)))
}

/// A card with `candidate` for subtypes may be added to `selected`: it has a creature type all of them have.
fn may_join(selected: &[SubtypeSet], candidate: SubtypeSet) -> bool {
    Subtype::CREATURE_TYPES.iter().any(|&t| selected.iter().all(|s| s.has(t)) && candidate.has(t))
}

/// A largest set that shares a creature type: the candidates of the creature type the most of them have.
fn largest_sharing_a_creature_type(q: &dyn Query, choice: &Choice, _: u32) -> Vec<ObjectId> {
    let with = |t: Subtype| choice.candidates.iter().copied().filter(|&c| q.chars(c).subtypes.has(t)).collect::<Vec<_>>();
    Subtype::CREATURE_TYPES.iter().map(|&t| with(t)).max_by_key(Vec::len).unwrap_or_default()
}

const SHARE_A_CREATURE_TYPE: Part =
    set_part(&(share_a_creature_type as SelectFn), &(largest_sharing_a_creature_type as LargestFn), 0, "that share a creature type");

/// "Return two target creature cards that share a creature type from your graveyard to your hand": both move together
/// (CR 400.3). The pair is judged again as it resolves (CR 608.2b, with last-known information, `TargetSpec::together`); if one
/// card has left the graveyard the other is still returned (Unbury's ruling, 2025-11-17).
fn return_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<ObjectId> = [0, 1].into_iter().filter_map(|i| targets::object(g, item, i)).collect();
    g.return_to_hand(&cards);
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

#[cfg(test)]
mod tests {
    use super::{may_join, shares_a_creature_type};
    use cantrip_api::{Subtype, SubtypeSet};

    /// With a Goblin and an Elf chosen no type is shared, so no card may join them; with only the Goblin chosen a Goblin
    /// may join and an Elf may not; with nothing chosen any creature may.
    #[test]
    fn a_card_may_join_only_if_it_has_a_type_every_chosen_card_has() {
        let goblin = SubtypeSet::of(&[Subtype::Goblin]);
        let elf = SubtypeSet::of(&[Subtype::Elf]);
        let goblin_elf = SubtypeSet::of(&[Subtype::Goblin, Subtype::Elf]);
        assert!(may_join(&[], elf));
        assert!(may_join(&[goblin], goblin) && may_join(&[goblin], goblin_elf));
        assert!(!may_join(&[goblin], elf), "an Elf shares nothing with a Goblin");
        assert!(!may_join(&[goblin, elf], goblin_elf), "no type is on both chosen cards");
        assert!(shares_a_creature_type(&[goblin, goblin_elf]) && !shares_a_creature_type(&[goblin, elf]));
    }
}
