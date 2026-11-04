use cantrip_api::*;
use cantrip_mechanics::{attach, targets, triggers};
use cantrip_oracle as oracle;

/// "When this Aura enters, if enchanted creature has flying, this Aura deals 2 damage to that creature and this Aura gains "Enchanted creature loses flying."" (CR 608.2).
fn if_enchanted_creature_has_flying_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.source)
        .and_then(|o: ObjectId| q.state().obj(o).attached_to)
        .is_some_and(|candidate: ObjectId| {
            q.chars(candidate).keywords.has(Keyword::Flying)
        })
}

/// "When this Aura enters, if enchanted creature has flying, this Aura deals 2 damage to that creature and this Aura gains "Enchanted creature loses flying."" (CR 604.1).
fn granted_2_apply_0(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = Keywords(chars.keywords.0 & !Keywords::of(&[Keyword::Flying]).0);
}

/// Damage and the ability grant are one instruction (CR 608.2c); the source may have left before resolution (CR 113.7a).
fn if_enchanted_creature_has_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creature = g.obj(item.object).attached_to;
    instruction(g, |g| {
        if let Some(creature) = creature {
            g.damage(item.object, Target::Object(creature), 2);
        }
        if is_live(g, item.object) && g.is_on_battlefield(item.object) {
            g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0)), Duration::Indefinite);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::THIS_ENTERS,
                if_enchanted_creature_has_flying,
                "When this Aura enters, if enchanted creature has flying, this Aura deals 2 damage to that creature and this Aura gains \"Enchanted creature loses flying.\"",
            )
            .condition(if_enchanted_creature_has_flying_if),
    ],
    delayed: &[
        granted_static_ability(
            Layer::Ability,
            attach::attached_object,
            granted_2_apply_0,
            "Enchanted creature loses flying.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::EARTHBIND
};
