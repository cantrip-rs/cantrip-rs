use cantrip_api::*;
use cantrip_mechanics::{copy, delayed, prepare};
use cantrip_oracle as oracle;

/// "When you next cast an instant or sorcery spell this turn, copy that spell": the spell sets up a delayed triggered ability,
/// number 0 of its `delayed` list, that triggers once on the next such spell its controller casts this turn and ends with the
/// turn if none is cast (CR 603.7b). The copy of this spell (itself a sorcery, already cast) is not that next spell.
fn striking_palette(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::next_this_turn(g, item, 0, item.object);
    Ok(())
}

/// Pigment Wrangler's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static STRIKING_PALETTE: CardDef = CardDef {
    name: "Striking Palette",
    cost: mana("{R}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], striking_palette),
    delayed: &[copy::COPY_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST],
    text: "When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets for the copy.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&STRIKING_PALETTE),
    ..oracle::PIGMENT_WRANGLER
};

pub static EXTRA: &[&CardDef] = &[&STRIKING_PALETTE];
