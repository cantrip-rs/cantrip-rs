use cantrip_api::*;
use cantrip_mechanics::lands::count_you_control;
use cantrip_mechanics::statics::{self, plus_pt};
use cantrip_mechanics::{combat, library};
use cantrip_oracle as oracle;

/// "As long as you control ten or more lands, creatures you control get +2/+2" (CR 604.1, 611.3a) as the static's `affects`: each
/// creature its controller controls, while that controller controls ten or more lands, each judged as the earlier layers make it
/// and by control, not ownership (`lands::count_you_control`).
fn your_creatures_while_you_control_ten_or_more_lands(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::your_creatures(q, source, affected, so_far) && count_you_control(q, source, &|c| c.is(CardType::Land)) >= 10
}

/// "Whenever this creature or another creature you control with toughness greater than its power enters" (CR 603.6a): this creature,
/// whatever it is as it enters, or another creature under your control as it entered whose toughness is greater than its power as
/// it is on the battlefield (`combat::your_creature_with_toughness_greater_than_power`, which Tapestry Warden's rules read too).
/// The entering creature is the trigger's subject.
fn this_or_another_creature_you_control_with_toughness_greater_than_power_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield) && (*new == me.source || combat::your_creature_with_toughness_greater_than_power(q, me, *new)) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

/// "Look at the top card of your library. If it's a land card, you may put it onto the battlefield tapped. Otherwise, put it into your
/// hand" (CR 608.2c, 608.2d, 401.5): "otherwise" takes in a land card the player declines too (Bloomburrow release notes: "If the
/// top card of your library is a land card, you may choose to put it into your hand rather than onto the battlefield tapped"), so it
/// reads as Risen Reef's `library::look_at_top_land_onto_battlefield_else_hand`, tapped.
fn look_at_the_top_card_land_to_battlefield_otherwise_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::look_at_top_land_onto_battlefield_else_hand(g, item)
}

/// Reach is a core keyword.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        your_creatures_while_you_control_ten_or_more_lands,
        plus_pt::<2, 2>,
        "As long as you control ten or more lands, creatures you control get +2/+2.",
    )],
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, this_or_another_creature_you_control_with_toughness_greater_than_power_enters),
        look_at_the_top_card_land_to_battlefield_otherwise_hand,
        "Whenever this creature or another creature you control with toughness greater than its power enters, look at the top card of your library. If it's a land card, you may put it onto the battlefield tapped. Otherwise, put it into your hand.",
    )],
    ..oracle::FECUND_GREENSHELL
};
