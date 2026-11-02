use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Enchantment creatures you control have deathtouch, lifelink, and hexproof." (CR 604.1).
fn enchantment_creatures_control_have_deathtouch_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).is(CardType::Enchantment) && (*so_far).is(CardType::Creature)
        && asker.controller == q.controller(affected)
}

/// "Enchantment creatures you control have deathtouch, lifelink, and hexproof." (CR 604.1).
fn enchantment_creatures_control_have_deathtouch_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::Deathtouch);
    chars.keywords = chars.keywords.with(Keyword::Lifelink);
    chars.keywords = chars.keywords.with(Keyword::Hexproof);
}

/// "{1}{W}: Target non-Aura enchantment you control becomes a creature in addition to its other types and has base power and base toughness each equal to its mana value." (CR 115.1).
fn target_non_aura_enchantment_control_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((! (q
        .chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Aura))) && q
        .chars(id).is(CardType::Enchantment) && asker.controller == q.controller(id)))
    )
}

/// "{1}{W}: Target non-Aura enchantment you control becomes a creature in addition to its other types and has base power and base toughness each equal to its mana value." (CR 608.2, 611.2a).
fn target_non_aura_enchantment_control(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = targets::object(g, item, 0);
    let kind_0_0_0 = EffectKind::AddTypes(TypeSet::of(&[CardType::Creature]));
    let kind_0_1_0 = {
        let q: &dyn Query = g;
        EffectKind::SetPt(
            t_0
                .get(0)
                .copied()
                .and_then(|t: Target| t.object())
                .map_or(0, |o: ObjectId| i64::from(q.mana_value(o))) as i16,
            t_0
                .get(0)
                .copied()
                .and_then(|t: Target| t.object())
                .map_or(0, |o: ObjectId| i64::from(q.mana_value(o))) as i16,
        )
    };
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::Indefinite);
                g.add_effect(item.object, object, kind_0_1_0, Duration::Indefinite);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{W}"),
                target_non_aura_enchantment_control,
                "{1}{W}: Target non-Aura enchantment you control becomes a creature in addition to its other types and has base power and base toughness each equal to its mana value.",
            )
            .targets(&[target(target_non_aura_enchantment_control_target_0)]),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            enchantment_creatures_control_have_deathtouch_affects,
            enchantment_creatures_control_have_deathtouch_apply_0,
            "Enchantment creatures you control have deathtouch, lifelink, and hexproof.",
        ),
    ],
    ..oracle::ZUR_ETERNAL_SCHEMER
};
