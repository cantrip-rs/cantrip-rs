use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Two untapped creatures and/or lands you control" as the permanents the additional cost taps (CR 118.3, 601.2b): any untapped
/// creature or land its caster controls, a creature that came under their control this turn included (CR 302.6 limits only a
/// creature's own {T}, and this taps the permanents as a cost). A land that is also a creature is one choice, tapped once.
fn untapped_creature_or_land_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    let chars = q.chars(id);
    q.controller(id) == asker.controller && !q.obj(id).tapped && (chars.is(CardType::Creature) || chars.is(CardType::Land))
}

pub static CARD: CardDef = CardDef {
    // As an additional cost to cast this spell, tap two untapped creatures and/or lands you control.
    additional_cost: Some(Cost::tap_untapped(2, untapped_creature_or_land_you_control)),
    ..oracle::FEAR_OF_EXPOSURE
};
