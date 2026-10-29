use cantrip_api::*;
use cantrip_mechanics::triggers::{ANOTHER_CREATURE_YOU_CONTROL_ENTERS, THIS_ATTACKS};
use cantrip_mechanics::{battle_cry, effects};
use cantrip_oracle as oracle;

/// "Wingnut gains your choice of flying, menace, or haste until end of turn." (CR 608.2d, 611.2a, 702.9, 702.110, 702.10).
fn gain_a_keyword(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_your_choice_until_end_of_turn(g, item, &CARD, item.object, &[Keyword::Flying, Keyword::Menace, Keyword::Haste])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
            gain_a_keyword,
            "Alliance — Whenever another creature you control enters, Wingnut gains your choice of flying, menace, or haste until end of turn.",
        ),
        triggered(THIS_ATTACKS, battle_cry::pump_other_attackers, "Whenever Wingnut attacks, each other attacking creature gets +1/+0 until end of turn."),
    ],
    labels: &["Flying", "Menace", "Haste"],
    ..oracle::WINGNUT_BAT_ON_THE_BELFRY
};
