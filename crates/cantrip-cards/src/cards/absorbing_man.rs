use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Up to one target artifact, non-Aura enchantment, or land" (CR 115.1, 115.4): a permanent on the battlefield with one of
/// those types that is not an Aura.
fn artifact_non_aura_enchantment_or_land(q: &dyn Query, _: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    if !q.is_on_battlefield(id) {
        return false;
    }
    let chars = q.chars(id);
    chars.is(CardType::Artifact) || chars.is(CardType::Land) || (chars.is(CardType::Enchantment) && !chars.has_subtype(Subtype::Aura))
}

/// "Until your next turn, Absorbing Man becomes a copy of up to one target artifact, non-Aura enchantment, or land, except his
/// name is Absorbing Man, he's a legendary 4/4 Human Villain creature in addition to his other types, and he has vigilance"
/// (CR 707.2, 707.9b, 611.2b). The target's copiable values as the trigger resolves, with those exceptions, so they are
/// copiable values too (CR 707.9a-b); "Human" and "Villain" are words of his text (CR 612). It lasts until his controller's next
/// turn begins, before their first main phase, so he is Absorbing Man again when this triggers next. With no target, or one no
/// longer legal (CR 608.2b), he stays as he is.
fn become_a_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let words = g.words(item.object);
    let values = g
        .copiable_values(target)
        .named("Absorbing Man")
        .legendary()
        .sized(4, 4)
        .with_type(CardType::Creature)
        .with_subtype(words.subtype(Subtype::Human))
        .with_subtype(words.subtype(Subtype::Villain))
        .with_keyword(Keyword::Vigilance);
    g.become_copy(item.object, item.object, values, Duration::UntilNextTurnOf(item.controller));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_FIRST_MAIN_PHASE,
        become_a_copy,
        "At the beginning of your first main phase, until your next turn, Absorbing Man becomes a copy of up to one target artifact, non-Aura enchantment, or land, except his name is Absorbing Man, he's a legendary 4/4 Human Villain creature in addition to his other types, and he has vigilance.",
    )
    .targets(&[target(artifact_non_aura_enchantment_or_land).optional()])],
    ..oracle::ABSORBING_MAN
};
