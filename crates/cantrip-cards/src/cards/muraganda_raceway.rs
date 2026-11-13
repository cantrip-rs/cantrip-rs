use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    // "Max speed — {T}: Add {C}{C}" (CR 702.178a): a second mana ability, granted by the static below while its controller's
    // speed is 4 (CR 605.1a), as Endrider Catalyzer's "Max speed — {T}: Add {R}{R}".
    delayed: &[mana_ability(TAP, add_n(Mana::C, 2), "{T}: Add {C}{C}.")],
    statics: &[max_speed("Max speed \u{2014} {T}: Add {C}{C}.")],
    ..oracle::MURAGANDA_RACEWAY
};
