use cantrip_api::*;
use cantrip_mechanics::{copy, delayed, targets};
use cantrip_oracle as oracle;

/// "Create a token that's a copy of another target creature you control, except it's a 1/1 red Balloon creature in addition to its
/// other colors and types and it has flying and haste. Sacrifice it at the beginning of the next end step" (CR 707.2, 707.9b, 111.1,
/// 603.7). The exceptions are copiable values, so a copy of the token is a 1/1 red Balloon with flying and haste too (and is not
/// sacrificed by this ability); red is added to the colors the copied values have (CR 105.2), not a replacement of them. Nothing
/// happens if the target is no longer legal (CR 608.2b). The delayed ability is about every token made (CR 603.7c).
fn create_balloon_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let plain = g.copiable_values(creature).sized(1, 1);
    let colors = plain.colors.unwrap_or(g.card(plain.card).colors());
    let words = g.words(item.object);
    let values = plain
        .colored(colors.with(words.color(Color::Red)))
        .with_subtype(words.subtype(Subtype::Balloon))
        .with_type(CardType::Creature)
        .with_keyword(Keyword::Flying)
        .with_keyword(Keyword::Haste);
    let tokens = g.create_token_copies_of(values, item.controller, 1)?;
    g.end_sentence();
    delayed::at_next_end_step_about(g, item, 0, &tokens);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(TAP),
        create_balloon_copy,
        "{1}, {T}: Create a token that's a copy of another target creature you control, except it's a 1/1 red Balloon creature in addition to its other colors and types and it has flying and haste. Sacrifice it at the beginning of the next end step. Activate only as a sorcery.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])
    .sorcery_speed()],
    delayed: &[copy::SACRIFICE_IT_AT_END_STEP],
    ..oracle::THE_JOLLY_BALLOON_MAN
};
