use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target player discards a card. Draw a card at the beginning of the next turn's upkeep." (CR 608.2, 121.1).
fn delayed_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)
}

/// "Target player discards a card. Draw a card at the beginning of the next turn's upkeep." (CR 608.2, 701.9a).
fn target_player_discards_card_draw(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    effects::target_player_discards::<1>(g, item)?;
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::PLAYER],
        target_player_discards_card_draw,
    ),
    delayed: &[delayed_trigger(delayed_2, "Draw a card at the beginning of the next turn's upkeep.")],
    ..oracle::MIND_RAVEL
};
