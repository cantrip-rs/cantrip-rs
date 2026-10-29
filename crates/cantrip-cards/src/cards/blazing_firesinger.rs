use cantrip_api::*;
use cantrip_mechanics::prepare;
use cantrip_oracle as oracle;

/// "Add {R}{R}{R}{R}{R}": a spell's effect, not a mana ability (CR 605.1a), so it resolves from the stack as the copy is cast
/// (CR 722.3c) and the mana empties at the end of the step (CR 500.4).
fn seething_song(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 5);
    Ok(())
}

/// Blazing Firesinger's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static SEETHING_SONG: CardDef = CardDef {
    name: "Seething Song",
    cost: mana("{2}{R}"),
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], seething_song),
    text: "Add {R}{R}{R}{R}{R}.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&SEETHING_SONG),
    ..oracle::BLAZING_FIRESINGER
};

pub static EXTRA: &[&CardDef] = &[&SEETHING_SONG];
