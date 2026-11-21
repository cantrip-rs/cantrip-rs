use cantrip_api::*;
use cantrip_mechanics::learn::lesson_cards_in_graveyard;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, mill two cards" (CR 701.17).
fn mill_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 2);
    Ok(())
}

/// "As long as there is a Lesson card in your graveyard, this creature can attack as though it didn't have defender" (CR 702.3b,
/// 609.4): only this creature, and only while its controller's graveyard holds a Lesson card. It keeps defender for everything
/// else. Defender is a core keyword read from the card's characteristics.
fn may_attack_with_a_lesson_in_your_graveyard(q: &dyn Query, asker: &Asker, creature: ObjectId) -> bool {
    creature == asker.source && lesson_cards_in_graveyard(q, asker.controller) > 0
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, mill_two, "When this creature enters, mill two cards.")],
    attacks_despite_defender: &[may_attack_with_a_lesson_in_your_graveyard],
    ..oracle::PLATYPUS_BEAR
};
