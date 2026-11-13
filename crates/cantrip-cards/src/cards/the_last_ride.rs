use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::effects;
use cantrip_mechanics::statics::itself;
use cantrip_oracle as oracle;

/// "The Last Ride gets -X/-X, where X is your life total" (CR 613.4c, layer 7c, 604.1): its own static ability, working whether or
/// not it is a creature, with "your" the controller of the Vehicle as the layers are asked again (X follows life as it changes,
/// CR 611.3a). Its printed 13/13 is the base that the subtraction reduces; a negative life total makes X 0 (CR 107.1b, as Death's
/// Shadow's ruling has it), so it never grows past 13/13.
fn minus_x_x_where_x_is_your_life_total(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = q.state();
    let x = state.player(state.obj(source).controller).life.max(0);
    chars.power = chars.power.map(|v| v - x);
    chars.toughness = chars.toughness.map(|v| v - x);
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself,
        minus_x_x_where_x_is_your_life_total,
        "The Last Ride gets -X/-X, where X is your life total.",
    )],
    abilities: &[
        ability(cost("{2}{B}").and(life(2)), |g, item| effects::draw(g, item, 1), "{2}{B}, Pay 2 life: Draw a card."),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::THE_LAST_RIDE
};
