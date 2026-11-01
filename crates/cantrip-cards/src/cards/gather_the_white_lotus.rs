use cantrip_api::*;
use cantrip_mechanics::{effects, permanents_with_subtype_you_control, tokens};
use cantrip_oracle as oracle;

/// "Create a 1/1 white Ally creature token for each Plains you control. Scry 2." (CR 111.1, 701.22): the Plains counted as it
/// resolves, through the layers, by the word its text says now (CR 612.2: Magical Hack can make it count Islands); the tokens
/// are made together, then the scry.
fn tokens_for_each_plains_then_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let plains = g.words(item.object).subtype(Subtype::Plains);
    let count = permanents_with_subtype_you_control(g, item.controller, plains) as u32;
    tokens::create(g, item.controller, &tokens::ALLY, count)?;
    effects::scry_two(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], tokens_for_each_plains_then_scry),
    ..oracle::GATHER_THE_WHITE_LOTUS
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
