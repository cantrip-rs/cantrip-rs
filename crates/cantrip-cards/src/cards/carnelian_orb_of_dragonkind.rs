use cantrip_api::*;
use cantrip_oracle as oracle;

/// "If that mana is spent on a Dragon creature spell, it gains haste until end of turn" (CR 106.6): an effect of the mana
/// itself, no triggered ability (no "when", CR 603.1), so nothing goes on the stack and nothing can counter it. The spell is a
/// Dragon and a creature as it is on the stack while the mana is spent (CR 601.2h), Dragon read through this card's words as
/// the mana is added (CR 612.2).
pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add(Mana::R).only_for(ManaRestriction::SubtypeGainsHaste(SubtypeSet::of(&[Subtype::Dragon]))),
        "{T}: Add {R}. If that mana is spent on a Dragon creature spell, it gains haste until end of turn.",
    )],
    ..oracle::CARNELIAN_ORB_OF_DRAGONKIND
};
