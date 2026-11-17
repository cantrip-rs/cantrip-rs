use cantrip_api::*;
use cantrip_mechanics::history::from_the_future;
use cantrip_mechanics::{copy, delayed, effects, plot, triggers};
use cantrip_oracle as oracle;

/// "\u{2212}6: Until end of turn, whenever you cast a spell, copy it." (CR 603.7b): sets up a delayed triggered ability for the
/// turn, controlled by the ability's controller.
fn copy_spells_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::this_turn(g, item, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cast_only_if: Some(from_the_future),
    abilities: &[
        ability(loyalty(1), effects::loot, "+1: Draw a card, then discard a card."),
        ability(loyalty(1), plot::may_exile_a_cheap_nonland_card_from_hand_and_plot, "+1: You may exile a nonland card with mana value 3 or less from your hand. If you do, it becomes plotted."),
        ability(loyalty(-6), copy_spells_this_turn, "\u{2212}6: Until end of turn, whenever you cast a spell, copy it. You may choose new targets for the copy."),
    ],
    delayed: &[triggered(
        when_asked(EventMask::SPELL_CAST, triggers::you_cast_a_spell_about_it),
        copy::copy_the_cast_spell,
        "Whenever you cast a spell, copy it. You may choose new targets for the copy.",
    )],
    ..oracle::JACE_REAWAKENED
};
