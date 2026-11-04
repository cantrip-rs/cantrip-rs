use cantrip_api::*;
use cantrip_mechanics::{effects, mana, targets};
use cantrip_oracle as oracle;

/// "Target artifact and/or enchantment cards from your graveyard" (CR 115.1), up to two: each an artifact card or an
/// enchantment card, or both.
fn artifact_or_enchantment_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Artifact, CardType::Enchantment])
}

/// The Adventure (CR 715): a sorcery, then exiled on its adventure, from where the land may be played (CR 715.3d).
pub static FAITH_GRIEF: CardDef = CardDef {
    spell: spell(&[up_to(2, artifact_or_enchantment_card_in_your_graveyard)], effects::return_targets_to_hand),
    ..oracle::FAITH_GRIEF
};

pub static CARD: CardDef = CardDef {
    // "This land enters tapped" (CR 614.1d).
    enters: Some(mana::enters_tapped),
    abilities: &[mana_ability(TAP, add(Mana::W), "{T}: Add {W}.")],
    adventure: Some(&FAITH_GRIEF),
    ..oracle::ISHGARD_THE_HOLY_SEE
};

pub static EXTRA: &[&CardDef] = &[&FAITH_GRIEF];
