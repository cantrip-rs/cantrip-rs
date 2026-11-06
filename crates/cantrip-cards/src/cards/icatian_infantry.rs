use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// This creature gains banding until end of turn (CR 611.2a, 702.22).
fn gains_banding(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::this_gains_until_end_of_turn(g, item, Keyword::Banding)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}"), effects::this_gains_first_strike, "{1}: This creature gains first strike until end of turn."),
        ability(cost("{1}"), gains_banding, "{1}: This creature gains banding until end of turn."),
    ],
    ..oracle::ICATIAN_INFANTRY
};
