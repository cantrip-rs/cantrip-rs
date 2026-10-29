use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, create a Heartwood token." (CR 603.2, 701.7a).
fn create_heartwood(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_heartwood(g, item.controller)
}

/// "Put a +1/+1 counter on this creature. It gains your choice of trample, hexproof, or haste until end of turn."
/// (CR 122.1, 608.2d, 611.2a, 702.10, 702.11, 702.19).
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    g.end_sentence();
    effects::gain_your_choice_until_end_of_turn(g, item, &CARD, item.object, &[Keyword::Trample, Keyword::Hexproof, Keyword::Haste])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_heartwood, "When this creature enters, create a Heartwood token."),
        ability(
            cost("{1}").and(sacrifice(1, targets::filters::another_artifact)),
            grow,
            "{1}, Sacrifice another artifact: Put a +1/+1 counter on this creature. It gains your choice of trample, hexproof, or haste until end of turn.",
        ),
    ],
    labels: &["Trample", "Hexproof", "Haste"],
    ..oracle::HUNGERING_PUPPETBEAST
};

pub static EXTRA: &[&CardDef] = &[&tokens::HEARTWOOD];
