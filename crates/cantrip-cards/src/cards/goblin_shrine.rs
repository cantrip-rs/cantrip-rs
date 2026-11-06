use cantrip_api::*;
use cantrip_oracle as oracle;

/// "As long as enchanted land is a basic Mountain, Goblin creatures get +1/+0." (CR 604.1).
fn as_long_as_enchanted_land_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (Some(asker.source)
        .and_then(|o: ObjectId| q.state().obj(o).attached_to)
        .is_some_and(|candidate: ObjectId| {
            q.chars(candidate).supertypes.basic
                && q
                    .chars(candidate)
                    .has_subtype(q.words(asker.source).subtype(Subtype::Mountain))
        }))
        && (((*so_far).has_subtype(q.words(asker.source).subtype(Subtype::Goblin))
            && (*so_far).is(CardType::Creature)))
}

/// "As long as enchanted land is a basic Mountain, Goblin creatures get +1/+0." (CR 604.1).
fn as_long_as_enchanted_land_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    let (p, t) = (1i32, 0i32);
    chars.power = chars.power.map(|v| v.saturating_add(p));
    chars.toughness = chars.toughness.map(|v| v.saturating_add(t));
}

/// "When this Aura leaves the battlefield, it deals 1 damage to each Goblin creature." (CR 608.2, 120.3).
fn deals_1_damage_goblin_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let recipients_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).has_subtype(q.words(item.object).subtype(Subtype::Goblin))
                    && q.chars(id).is(CardType::Creature)
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .map(Target::Object)
            .collect::<Vec<Target>>()
    };
    let batch: Vec<(ObjectId, Target, u32)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, 1))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            cantrip_mechanics::triggers::THIS_LEAVES,
            deals_1_damage_goblin_creature,
            "When this Aura leaves the battlefield, it deals 1 damage to each Goblin creature.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            as_long_as_enchanted_land_affects,
            as_long_as_enchanted_land_apply_0,
            "As long as enchanted land is a basic Mountain, Goblin creatures get +1/+0.",
        ),
    ],
    enchant: Some(cantrip_mechanics::targets::LAND),
    ..oracle::GOBLIN_SHRINE
};
