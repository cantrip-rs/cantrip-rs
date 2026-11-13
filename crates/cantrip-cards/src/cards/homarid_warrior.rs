use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{U}"),
            effects::this_gains_shroud_doesnt_untap_next_untap_step_then_taps,
            "{U}: This creature gains shroud until end of turn and doesn't untap during your next untap step. Tap it.",
        ),
    ],
    ..oracle::HOMARID_WARRIOR
};
