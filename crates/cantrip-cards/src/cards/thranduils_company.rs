use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counters_on_first_target_that_gains_until_end_of_turn;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "As long as you control another Elf" (CR 604.1, 305.2): asked of the player who controls this creature, each time lands are
/// counted, so the extra land play comes and goes with the Elf. Another permanent than this one with the creature type Elf, read
/// through the words of this card's text (CR 612); an opponent's Elf does not count.
fn you_control_another_elf(q: &dyn Query, asker: &Asker) -> bool {
    let elf = q.words(asker.source).subtype(Subtype::Elf);
    q.permanents_of(asker.controller).into_iter().any(|id| id != asker.source && q.chars(id).has_subtype(elf))
}

pub static CARD: CardDef = CardDef {
    // "You may play an additional land on each of your turns" (CR 305.2), cumulative with other such effects (the rulings).
    play_rules: &[PlayRule::While(you_control_another_elf, &[PlayRule::ExtraLandPlays(1)])],
    abilities: &[triggered(
        triggers::LANDFALL,
        plus_one_counters_on_first_target_that_gains_until_end_of_turn::<2, { Keywords::of(&[Keyword::Vigilance]).0 }>,
        "Landfall \u{2014} Whenever a land you control enters, put two +1/+1 counters on target creature you control. It gains vigilance until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::THRANDUILS_COMPANY
};
