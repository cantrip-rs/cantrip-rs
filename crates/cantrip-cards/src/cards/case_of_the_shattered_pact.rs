use cantrip_api::*;
use cantrip_mechanics::triggers::{BEGINNING_OF_YOUR_COMBAT, THIS_ENTERS};
use cantrip_mechanics::{effects, library, select, solved, targets, vivid};
use cantrip_oracle as oracle;

/// "Search your library for a basic land card, reveal it, put it into your hand, then shuffle" (CR 701.23).
fn search_basic_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, select::BASIC_LAND_CARD, true)?;
    Ok(())
}

/// "To solve - There are five colors among permanents you control" (CR 719.3a, 105.1): an intervening "if", true as the end step
/// begins and as the ability resolves. Colorless is no color, so all five colors of Magic have to be there.
fn five_colors_among_your_permanents(q: &dyn Query, asker: &Asker) -> bool {
    vivid::count(q, asker.controller) == 5
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, search_basic_land, "When this Case enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle."),
        solved::to_solve(
            five_colors_among_your_permanents,
            "To solve \u{2014} There are five colors among permanents you control. (If unsolved, solve at the beginning of your end step.)",
        ),
        solved::triggered(
            BEGINNING_OF_YOUR_COMBAT,
            effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Flying, Keyword::DoubleStrike, Keyword::Vigilance]).0 }>,
            "Solved \u{2014} At the beginning of combat on your turn, target creature you control gains flying, double strike, and vigilance until end of turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    ..oracle::CASE_OF_THE_SHATTERED_PACT
};
