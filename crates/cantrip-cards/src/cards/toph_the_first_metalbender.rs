use cantrip_api::*;
use cantrip_mechanics::statics::your_artifacts;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::earthbend;
use cantrip_oracle as oracle;

/// "Nontoken artifacts you control" (CR 613.1d): artifacts under the source's controller's control that are not tokens, judged on
/// what the earlier layers made them.
fn your_nontoken_artifacts(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    !q.state().obj(affected).token && your_artifacts(q, source, affected, so_far)
}

/// "... are lands in addition to their other types" (CR 205.1b, layer 4): the land type is added and every other type kept. It
/// gives no mana ability (the reminder text), and no land subtype.
fn land_too(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.types = chars.types.with(CardType::Land);
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Type, your_nontoken_artifacts, land_too, "Nontoken artifacts you control are lands in addition to their other types. (They don't gain the ability to {T} for mana.)")],
    abilities: &[triggered(
        YOUR_END_STEP,
        earthbend::effect::<2>,
        "At the beginning of your end step, earthbend 2. (Target land you control becomes a 0/0 creature with haste that's still a land. Put two +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped.)",
    )
    .targets(&[earthbend::TARGET])],
    delayed: &[earthbend::RETURN],
    ..oracle::TOPH_THE_FIRST_METALBENDER
};
