use cantrip_api::*;
use cantrip_oracle as oracle;

/// "All creatures lose flying." (CR 604.1).
fn all_creatures_lose_flying_affects(
    _: &dyn LayerQuery,
    _: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    (*so_far).is(CardType::Creature)
}

/// "All creatures lose flying." (CR 604.1).
fn all_creatures_lose_flying_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = Keywords(chars.keywords.0 & !Keywords::of(&[Keyword::Flying]).0);
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            all_creatures_lose_flying_affects,
            all_creatures_lose_flying_apply_0,
            "All creatures lose flying.",
        ),
    ],
    ..oracle::GRAVITY_SPHERE
};
