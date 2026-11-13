use cantrip_api::*;
use cantrip_mechanics::statics::{self, grant_delayed_ability};
use cantrip_mechanics::triggers::EACH_END_STEP;
use cantrip_mechanics::{effects, history, targets};
use cantrip_oracle as oracle;

const ARTIFACTS_ARE_FOODS: &str =
    "Artifacts you control are Foods in addition to their other types and have \"{2}, {T}, Sacrifice this artifact: You gain 3 life.\"";

/// "Artifacts you control" (CR 613.1d, 613.1f): every artifact under this creature's controller's control, Ragost itself when it
/// is one, as the earlier layers made it - an artifact only because of another effect counts, by control, not ownership.
fn your_artifacts(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Artifact) && q.controller(affected) == q.controller(source)
}

/// Layer 4 (CR 613.1d, 205.3g): "are Foods in addition to their other types" - the artifact type Food added; the artifacts keep
/// every type, subtype and ability they have (the ruling).
fn foods_too(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.subtypes = chars.subtypes.with(Subtype::Food);
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}, {T}").and(sacrifice(1, targets::filters::food)),
            effects::damage_each_opponent_n::<3>,
            "{1}, {T}, Sacrifice a Food: Ragost deals 3 damage to each opponent.",
        ),
        // "At the beginning of each end step, if you gained life this turn" (CR 603.4): an intervening "if", checked as the
        // step begins and again as the ability resolves; the check is of this turn's life gained by its controller.
        triggered(EACH_END_STEP, effects::untap_self, "At the beginning of each end step, if you gained life this turn, untap Ragost.")
            .condition(history::you_gained_life_this_turn),
    ],
    delayed: &[ability(
        cost("{2}, {T}").and(SACRIFICE_THIS),
        effects::gain_three_life,
        "{2}, {T}, Sacrifice this artifact: You gain 3 life.",
    )],
    statics: &[
        static_ability(Layer::Type, your_artifacts, foods_too, ARTIFACTS_ARE_FOODS),
        // CR 613.6: the effect that began in layer 4 applies to the same artifacts in layer 6.
        static_ability(Layer::Ability, statics::itself, grant_delayed_ability, ARTIFACTS_ARE_FOODS).continues(),
    ],
    ..oracle::RAGOST_DEFT_GASTRONAUT
};
