use cantrip_api::*;
use cantrip_mechanics::{solved, targets};
use cantrip_oracle as oracle;

/// "To solve — You control seven or more lands" (CR 719.3a): an intervening "if" (CR 603.4), asked as the end step begins and
/// as the ability resolves.
fn seven_or_more_lands(q: &dyn Query, asker: &Asker) -> bool {
    q.permanents_of(asker.controller).into_iter().filter(|&id| q.chars(id).is(CardType::Land)).count() >= 7
}

pub static CARD: CardDef = CardDef {
    abilities: &[solved::to_solve(
        seven_or_more_lands,
        "To solve \u{2014} You control seven or more lands. (If unsolved, solve at the beginning of your end step.)",
    )],
    // "You may play an additional land on each of your turns" (CR 305.2), and "Solved — You may look at the top card of your
    // library any time, and you may play lands and cast creature and enchantment spells from the top of your library" (CR
    // 702.169b: as long as this Case is solved; CR 401.2, 305.1, 601.3), for their mana costs.
    play_rules: &[
        PlayRule::ExtraLandPlays(1),
        solved::play_rules(&[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopLands, PlayRule::LibraryTopSpells(targets::filters::creature_or_enchantment, None)]),
    ],
    ..oracle::CASE_OF_THE_LOCKED_HOTHOUSE
};
