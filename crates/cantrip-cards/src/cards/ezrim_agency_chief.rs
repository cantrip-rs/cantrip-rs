use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Investigate twice." (CR 701.16a)
fn investigate_twice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)?;
    tokens::investigate(g, item.controller)
}

/// "Ezrim gains your choice of vigilance, lifelink, or hexproof until end of turn." (CR 608.2d, 611.2a, 702.20, 702.15, 702.11).
fn gain_a_keyword(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_your_choice_until_end_of_turn(g, item, &CARD, item.object, &[Keyword::Vigilance, Keyword::Lifelink, Keyword::Hexproof])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, investigate_twice, "When Ezrim enters, investigate twice."),
        ability(
            cost("{1}").and(sacrifice(1, targets::filters::artifact)),
            gain_a_keyword,
            "{1}, Sacrifice an artifact: Ezrim gains your choice of vigilance, lifelink, or hexproof until end of turn.",
        ),
    ],
    labels: &["Vigilance", "Lifelink", "Hexproof"],
    ..oracle::EZRIM_AGENCY_CHIEF
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
