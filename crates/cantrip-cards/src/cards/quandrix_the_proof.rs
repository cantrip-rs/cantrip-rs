use cantrip_api::*;
use cantrip_mechanics::{statics, triggers};
use cantrip_oracle as oracle;

/// "Instant and sorcery spells you cast from your hand" (CR 601.2i, 707.10): a spell on the stack that was cast, not a
/// copy of one, from its controller's hand, that is an instant or a sorcery as it is on the stack.
fn instant_or_sorcery_spell_you_cast_from_your_hand(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    (chars.is(CardType::Instant) || chars.is(CardType::Sorcery))
        && view.state().obj(affected).cast_from == Some(ZoneKind::Hand)
        && statics::spell_you_cast(view, source, affected)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggers::CASCADE],
    delayed: &[triggers::CASCADE],
    statics: &[static_ability(
        Layer::Ability,
        instant_or_sorcery_spell_you_cast_from_your_hand,
        statics::grant_delayed_ability,
        "Instant and sorcery spells you cast from your hand have cascade.",
    )
    .everywhere()],
    ..oracle::QUANDRIX_THE_PROOF
};
