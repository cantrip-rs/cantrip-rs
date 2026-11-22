use cantrip_api::*;
use cantrip_mechanics::{card_types_in_graveyard, protection, targets, triggers};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast for each card type among cards in your graveyard" (CR 601.2f, 118.7a): counted as the
/// cost is determined, each type once however many cards have it. Only the cost of casting it; its mana value stays 13.
fn less_per_card_type_in_your_graveyard(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(card_types_in_graveyard(q, player) as u32)
}

/// "When you cast this spell, you gain control of target opponent during that player's next turn. After that turn, that
/// player takes an extra turn." Triggers from the stack (CR 603.2), so it resolves even if Emrakul is countered; a target
/// that is no longer legal does nothing (CR 608.2b).
fn control_then_extra_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Some(player) = item.target(0).player()
    {
        g.control_next(item, player, ControlWindow::Turn { then_extra_turn: true });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[less_per_card_type_in_your_graveyard],
    abilities: &[triggered(
        triggers::CAST_THIS_SPELL,
        control_then_extra_turn,
        "When you cast this spell, you gain control of target opponent during that player's next turn. After that turn, that player takes an extra turn.",
    )
    .targets(&[targets::OPPONENT])],
    statics: &[protection::from_type::<{ CardType::Instant as u8 }>("Protection from instants")],
    ..oracle::EMRAKUL_THE_PROMISED_END
};
