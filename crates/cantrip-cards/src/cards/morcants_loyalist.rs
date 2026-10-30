use cantrip_api::*;
use cantrip_mechanics::statics::{other_of_subtype_you_control, plus_pt};
use cantrip_mechanics::{lands, targets, triggers};
use cantrip_oracle as oracle;

/// "Another target Elf card from your graveyard" (CR 115.1, 601.2c): a card of your graveyard that is an Elf, other than this
/// card itself (the new object it became, CR 400.7, in that graveyard as the trigger is put on the stack). "Elf" is a word of this card's text (CR 612).
fn another_elf_card_in_your_graveyard(q: &dyn Query, me: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, me, t, &[])
        && matches!(t, Target::Object(id) if q.found_after_move(me.source) != Some(id) && q.chars(id).has_subtype(q.words(me.source).subtype(Subtype::Elf)))
}

/// "When this creature dies, return another target Elf card from your graveyard to your hand." (CR 400.7: it becomes a new
/// object.)
fn return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.return_to_hand(&[card]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_DIES, return_it, "When this creature dies, return another target Elf card from your graveyard to your hand.")
        .targets(&[target(another_elf_card_in_your_graveyard)])],
    statics: &[static_ability(
        Layer::PtModify,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Elf) }>,
        plus_pt::<1, 1>,
        "Other Elves you control get +1/+1.",
    )],
    ..oracle::MORCANTS_LOYALIST
};
