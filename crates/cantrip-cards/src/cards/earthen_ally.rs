use cantrip_api::*;
use cantrip_mechanics::statics::itself;
use cantrip_mechanics::earthbend;
use cantrip_oracle as oracle;

/// "This creature gets +1/+0 for each color among Allies you control" (CR 613.4c, 105.2): the colors, as the layers make them
/// now, of the Allies under this creature's controller's control (itself too, an Ally), each color counted once however many
/// of them have it, and colorless none.
fn plus_power_per_color_among_allies_you_control(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = q.state();
    let you = state.obj(source).controller;
    let ally = q.words(source).subtype(Subtype::Ally);
    let colors = state
        .battlefield()
        .iter()
        .filter(|&&id| state.obj(id).controller == you && q.chars(id).has_subtype(ally))
        .fold(ColorSet::NONE, |set, &id| ColorSet(set.0 | q.chars(id).colors.0));
    chars.power = chars.power.map(|power| power + colors.iter().count() as i32);
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, itself, plus_power_per_color_among_allies_you_control, "This creature gets +1/+0 for each color among Allies you control.")],
    abilities: &[ability(
        cost("{2}{W}{U}{B}{R}{G}"),
        earthbend::effect::<5>,
        "{2}{W}{U}{B}{R}{G}: Earthbend 5. (Target land you control becomes a 0/0 creature with haste that's still a land. Put five +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped.)",
    )
    .targets(&[earthbend::TARGET])],
    delayed: &[earthbend::RETURN],
    ..oracle::EARTHEN_ALLY
};
