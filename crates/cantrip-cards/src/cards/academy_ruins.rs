use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
                cost("{1}{U}").and(TAP),
                effects::put_first_target_on_top_of_library,
                "{1}{U}, {T}: Put target artifact card from your graveyard on top of your library.",
            )
            .targets(&[targets::ARTIFACT_CARD_IN_YOUR_GRAVEYARD]),
    ],
    ..oracle::ACADEMY_RUINS
};
