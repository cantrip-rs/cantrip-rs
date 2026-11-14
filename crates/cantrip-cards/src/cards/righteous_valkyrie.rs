use cantrip_api::*;
use cantrip_mechanics::effects::gain_life_equal_to_subject_toughness;
use cantrip_mechanics::statics::{plus_pt, your_creatures_while_seven_above_starting_life};
use cantrip_mechanics::{lands, triggers};
use cantrip_oracle as oracle;

/// "Whenever another Angel or Cleric you control enters" (CR 603.6a): another permanent under the controller's control as it
/// entered that is an Angel or a Cleric (a token too), the types read through the words of this card's text (CR 612); the
/// trigger is about it, so "that creature's toughness" can be read. Once for the permanent, though it is both.
fn another_angel_or_cleric_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Angel) }>(q, me, event)
        .or_else(|| triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Cleric) }>(q, me, event))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_angel_or_cleric_enters),
        gain_life_equal_to_subject_toughness,
        "Whenever another Angel or Cleric you control enters, you gain life equal to that creature's toughness.",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        your_creatures_while_seven_above_starting_life,
        plus_pt::<2, 2>,
        "As long as you have at least 7 life more than your starting life total, creatures you control get +2/+2.",
    )],
    ..oracle::RIGHTEOUS_VALKYRIE
};
