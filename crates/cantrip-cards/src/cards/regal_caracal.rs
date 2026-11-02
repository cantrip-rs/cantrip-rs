use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Other Cats you control get +1/+1 and have lifelink." (CR 604.1).
fn other_cats_control_get_1_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(asker.source == affected)
        && ((*so_far).has_subtype(q.words(asker.source).subtype(Subtype::Cat))
            && asker.controller == q.controller(affected))
}

/// "Other Cats you control get +1/+1 and have lifelink." (CR 604.1).
fn other_cats_control_get_1_apply_1(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Lifelink);
}

/// "Other Cats you control get +1/+1 and have lifelink." (CR 604.1).
fn other_cats_control_get_1_apply_0(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    let (p, t) = (1i16, 1i16);
    chars.power = chars.power.map(|v| v.saturating_add(p));
    chars.toughness = chars.toughness.map(|v| v.saturating_add(t));
}

/// "When this creature enters, create two 1/1 white Cat creature tokens with lifelink." (CR 608.2, 111.1).
fn create_two_1_1_white(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_0 = Vec::new();
    let def_0_0 = g.card_id(&tokens::CAT_LIFELINK);
    tokens_0.extend(std::iter::repeat((def_0_0, item.controller)).take(2));
    let words_0 = g.words(item.object);
    g.create_tokens_worded(&tokens_0, false, None, words_0)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            create_two_1_1_white,
            "When this creature enters, create two 1/1 white Cat creature tokens with lifelink.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            other_cats_control_get_1_affects,
            other_cats_control_get_1_apply_1,
            "Other Cats you control get +1/+1 and have lifelink.",
        ),
        static_ability(
                Layer::PtModify,
                other_cats_control_get_1_affects,
                other_cats_control_get_1_apply_0,
                "Other Cats you control get +1/+1 and have lifelink.",
            )
            .continues(),
    ],
    ..oracle::REGAL_CARACAL
};

pub static EXTRA: &[&CardDef] = &[&tokens::CAT_LIFELINK];
