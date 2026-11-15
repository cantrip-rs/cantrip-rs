use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_DRAW_YOUR_SECOND_CARD;
use cantrip_mechanics::{copy, delayed, targets};
use cantrip_oracle as oracle;

/// "Seek out — Whenever you draw your second card each turn, create a token that's a copy of another target creature
/// you control, except it isn't legendary, it's an Illusion artifact in addition to its other types, and it has haste.
/// Exile that token at the beginning of the next end step." (CR 603.2, 121.2, 707.2, 707.9b, 613.1f, 603.7c)
///
/// The exceptions are the copy's copiable values (CR 707.2): nonlegendary, plus the artifact card type and the Illusion
/// subtype in addition to everything the original has, and haste. The delayed ability carries the token objects this
/// resolution made, so it exiles them and not a later permanent with the same name (CR 400.7, 603.7c). "Seek out" is a
/// flavor word with no rules meaning (CR 207.2d). Nothing if the target is no longer legal (CR 608.2b).
fn copy_as_illusion(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(creature)
        .nonlegendary()
        .with_type(CardType::Artifact)
        .with_subtype(g.words(item.object).subtype(Subtype::Illusion))
        .with_keyword(Keyword::Haste);
    copy::create_token_copy_exiled_at_next_end_step(g, item, values, 0)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_DRAW_YOUR_SECOND_CARD,
        copy_as_illusion,
        "Seek out \u{2014} Whenever you draw your second card each turn, create a token that's a copy of another target \
         creature you control, except it isn't legendary, it's an Illusion artifact in addition to its other types, and \
         it has haste. Exile that token at the beginning of the next end step.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    delayed: &[delayed_trigger(delayed::exile_subject, "Exile that token at the beginning of the next end step.")],
    ..oracle::REG_BARCLAY_FIDGETY_FANTASIST
};
