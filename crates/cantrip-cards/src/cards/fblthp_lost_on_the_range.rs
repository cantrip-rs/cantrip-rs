use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Ward {2}. You may look at the top card of your library any time. The top card of your library has plot. The plot cost is
/// equal to its mana cost. You may plot nonland cards from the top of your library." (CR 702.21a, 401.2, 702.170a, 116.2k):
/// the top card is plotted as a special action at sorcery timing, for its mana cost, X being 0 (CR 107.3g-h), or for its own plot
/// cost where it has one. Plotting it exiles it, so the next card is the new top card.
pub static CARD: CardDef = CardDef {
    abilities: &[triggers::ward::<2>("Ward {2}")],
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopPlot],
    ..oracle::FBLTHP_LOST_ON_THE_RANGE
};
