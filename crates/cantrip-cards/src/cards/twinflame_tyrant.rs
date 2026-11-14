use cantrip_api::*;
use cantrip_mechanics::replacements;
use cantrip_oracle as oracle;

/// "If a source you control would deal damage to an opponent or a permanent an opponent controls, it deals double that damage
/// instead" (CR 614.1a, 120.3): any source its controller controls, combat or not, once the damage is divided among its
/// recipients (the rulings), dealt to an opponent or a permanent an opponent controls. Damage already prevented to nothing is no
/// damage to double (CR 615.6). Still dealt by the original source, not by the Tyrant.
pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::Damage,
        replacements::damage_you_deal_to_an_opponent_or_their_permanent,
        replacements::double_that_damage,
        "deals double that damage instead",
    )],
    ..oracle::TWINFLAME_TYRANT
};
