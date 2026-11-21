use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets};
use cantrip_oracle as oracle;

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
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.between(2, 2).together(select::SHARE_A_CREATURE_TYPE)]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::UNBURY };
