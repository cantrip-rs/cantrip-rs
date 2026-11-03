use cantrip_api::*;
use cantrip_mechanics::{statics, tokens};
use cantrip_oracle as oracle;

/// "+1: Until end of turn, Gideon becomes a 5/5 Human Soldier Ally creature with indestructible that's still a planeswalker. Prevent all damage that would be dealt to him this turn. 0: Create a 2/2 white Knight Ally creature token." (CR 615.1).
fn made_2_applies(_: &dyn Query, _: &Asker, _: ObjectId, _: &DamageEvent) -> bool {
    true
}

/// "+1: Until end of turn, Gideon becomes a 5/5 Human Soldier Ally creature with indestructible that's still a planeswalker. Prevent all damage that would be dealt to him this turn." (CR 608.2, 611.2a).
fn gideon_becomes_5_5_human(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let kind_0_0_0 = EffectKind::AddTypes(TypeSet::of(&[CardType::Creature]));
    let kind_0_2_0 = {
        let q: &dyn Query = g;
        EffectKind::AddSubtype(q.words(item.object).subtype(Subtype::Human))
    };
    let kind_0_3_1 = {
        let q: &dyn Query = g;
        EffectKind::AddSubtype(q.words(item.object).subtype(Subtype::Soldier))
    };
    let kind_0_4_2 = {
        let q: &dyn Query = g;
        EffectKind::AddSubtype(q.words(item.object).subtype(Subtype::Ally))
    };
    let kind_0_5_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Indestructible]));
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::SetPt(5, 5),
                    Duration::EndOfTurn,
                );
                g.add_effect(item.object, object, kind_0_2_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_0_3_1, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_0_4_2, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_0_5_0, Duration::EndOfTurn);
            }
        },
    );
    g.end_sentence();
    {
        let to = Some(item.object)
            .map(Target::Object)
            .into_iter()
            .collect::<Vec<Target>>()
            .first()
            .copied();
        let about = to.and_then(|to: Target| to.object()).or(Some(item.object));
        if let (Some(to), Some(about)) = (to, about) {
            g.add_effect(
                item.object,
                about,
                EffectKind::Prevention {
                    card: item.ability.card,
                    index: 0,
                    to,
                    shield: Shield::All,
                },
                Duration::EndOfTurn,
            );
        }
    }
    Ok(())
}

/// "0: Create a 2/2 white Knight Ally creature token." (CR 608.2).
fn create_2_2_white_knight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_3 = Vec::new();
    let def_3_0 = g.card_id(&tokens::KNIGHT_ALLY);
    tokens_3.extend(std::iter::repeat((def_3_0, item.controller)).take(1));
    let words_3 = g.words(item.object);
    g.create_tokens_worded(&tokens_3, false, None, words_3)?;
    Ok(())
}

pub static EMBLEM: CardDef = CardDef {
    name: "Gideon, Ally of Zendikar emblem",
    has_cost: false,
    statics: &[statics::CREATURES_YOU_CONTROL_GET_PLUS_ONE],
    ..CardDef::BLANK
};

/// "−4: You get an emblem with "Creatures you control get +1/+1."" (CR 608.2).
fn get_emblem_creatures_control_get(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    tokens::create_emblem(g, item.controller, &EMBLEM);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                loyalty(1),
                gideon_becomes_5_5_human,
                "+1: Until end of turn, Gideon becomes a 5/5 Human Soldier Ally creature with indestructible that's still a planeswalker. Prevent all damage that would be dealt to him this turn.",
            )
            .sorcery_speed(),
        ability(
                loyalty(0),
                create_2_2_white_knight,
                "0: Create a 2/2 white Knight Ally creature token.",
            )
            .sorcery_speed(),
        ability(
                loyalty(-4),
                get_emblem_creatures_control_get,
                "−4: You get an emblem with \"Creatures you control get +1/+1.\"",
            )
            .sorcery_speed(),
    ],
    granted_preventions: &[
        GrantedPreventionDef {
            hint: "+1: Until end of turn, Gideon becomes a 5/5 Human Soldier Ally creature with indestructible that's still a planeswalker. Prevent all damage that would be dealt to him this turn.",
            damage: "damage",
            of_it: None,
            applies: made_2_applies,
            then: None,
            text: "Prevent all damage that would be dealt to him this turn.",
        },
    ],
    ..oracle::GIDEON_ALLY_OF_ZENDIKAR
};

pub static EXTRA: &[&CardDef] = &[&tokens::KNIGHT_ALLY, &EMBLEM];
