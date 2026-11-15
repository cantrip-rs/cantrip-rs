use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "{T}: Add one mana of any color.", the mana ability the permanents are given: each is its own source (CR 113.7).
    delayed: &[mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color.")],
    // "Other permanents you control have ..." (CR 613.1f, layer 6): every permanent of its controller's but A Realm Reborn
    // itself, whatever its type; a permanent that is not a creature or has summoning sickness taps for it only when the rules
    // allow ({T} abilities of creatures need no summoning sickness, CR 302.6).
    statics: &[static_ability(
        Layer::Ability,
        statics::other_permanents_you_control,
        statics::grant_delayed_ability,
        "Other permanents you control have \"{T}: Add one mana of any color.\"",
    )],
    ..oracle::A_REALM_REBORN
};
