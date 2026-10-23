use cantrip_api::*;
use cantrip_mechanics::statics::{have_split_second, your_spells_of_types};
use cantrip_oracle as oracle;

/// "Instant and sorcery spells you control have split second" (CR 702.61a, 613.1f, layer 6): a static ability granting the
/// keyword to an instant or sorcery spell on the stack controlled by Samut's controller, whatever casts it and wherever it
/// was cast from. The grant is continuous while Samut is on the battlefield and the spell on the stack (CR 611.3a, 613.1f:
/// the layers are worked out again whenever characteristics are computed), and it ends when either leaves. While the spell
/// has it, no player may cast a spell or activate a nonmana ability (CR 702.61a). A creature spell, and any spell an
/// opponent controls, gets nothing.
const INSTANT_OR_SORCERY: u16 = TypeSet::of(&[CardType::Instant, CardType::Sorcery]).0;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        your_spells_of_types::<INSTANT_OR_SORCERY>,
        have_split_second,
        "Instant and sorcery spells you control have split second.",
    )
    .everywhere()],
    ..oracle::SAMUT_TYRANT_OF_NAKTAMUN
};
