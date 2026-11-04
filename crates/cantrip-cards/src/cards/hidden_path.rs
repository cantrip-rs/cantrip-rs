use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::{lands, statics};

/// "Green creatures have forestwalk." (CR 604.1).
fn green_creatures_have_forestwalk_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).colors.has(q.words(asker.source).color(Color::Green))
        && (*so_far).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            green_creatures_have_forestwalk_affects,
            statics::have_landwalk::<{ lands::sub(Subtype::Forest) }>,
            "Green creatures have forestwalk.",
        ),
    ],
    ..oracle::HIDDEN_PATH
};
