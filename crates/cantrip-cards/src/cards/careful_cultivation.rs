use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Channel — {1}{G}, Discard this card: Create a 1/1 green Human Monk creature token with "{T}: Add {G}."" (CR 608.2).
fn create_1_1_green_human_enchant(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Creature)))
    )
}

/// "As long as enchanted permanent is a creature, it gets +1/+3 and has reach and "{T}: Add {G}{G}."" (CR 604.1, 701.3a).
fn add_g_g_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    affected: ObjectId,
    _: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (Some(asker.source)
        .and_then(|o: ObjectId| q.state().obj(o).attached_to)
        .is_some_and(|candidate: ObjectId| q.chars(candidate).is(CardType::Creature)))
        && (q.state().obj(source).attached_to == Some(affected))
}

/// "As long as enchanted permanent is a creature, it gets +1/+3 and has reach and "{T}: Add {G}{G}."" (CR 604.1).
fn add_g_g_apply_1(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let asker = &Asker::new(source, q.controller(source));
    chars.keywords = chars.keywords.with(Keyword::Reach);
    let card = {
        let o = q.state().obj(asker.source);
        o.copied_card().unwrap_or(o.card)
    };
    let _ = card;
    let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) else { return };
    chars.abilities[slot] = Some(
        AbilityRef::delayed(
            {
                let o = q.state().obj(asker.source);
                o.copied_card().unwrap_or(o.card)
            },
            0,
        ),
    );
}

/// "As long as enchanted permanent is a creature, it gets +1/+3 and has reach and "{T}: Add {G}{G}."" (CR 604.1).
fn add_g_g_apply_0(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    let (p, t) = (1i16, 3i16);
    chars.power = chars.power.map(|v| v.saturating_add(p));
    chars.toughness = chars.toughness.map(|v| v.saturating_add(t));
}

/// "Channel — {1}{G}, Discard this card: Create a 1/1 green Human Monk creature token with "{T}: Add {G}."" (CR 608.2, 111.1).
fn create_1_1_green_human(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_2 = Vec::new();
    let def_2_0 = g.card_id(&tokens::HUMAN_MONK);
    tokens_2.extend(std::iter::repeat((def_2_0, item.controller)).take(1));
    let words_2 = g.words(item.object);
    g.create_tokens_worded(&tokens_2, false, None, words_2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{G}").and(DISCARD_THIS),
                create_1_1_green_human,
                "Channel — {1}{G}, Discard this card: Create a 1/1 green Human Monk creature token with \"{T}: Add {G}.\"",
            )
            .from(ZoneKind::Hand),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            add_g_g_affects,
            add_g_g_apply_1,
            "As long as enchanted permanent is a creature, it gets +1/+3 and has reach and \"{T}: Add {G}{G}.\"",
        ),
        static_ability(
                Layer::PtModify,
                add_g_g_affects,
                add_g_g_apply_0,
                "As long as enchanted permanent is a creature, it gets +1/+3 and has reach and \"{T}: Add {G}{G}.\"",
            )
            .continues(),
    ],
    delayed: &[mana_ability(TAP, add_n(Mana::G, 2), "{T}: Add {G}{G}.")],
    enchant: Some(target(create_1_1_green_human_enchant)),
    ..oracle::CAREFUL_CULTIVATION
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN_MONK];
