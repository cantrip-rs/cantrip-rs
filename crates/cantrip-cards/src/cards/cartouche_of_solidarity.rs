use cantrip_api::*;
use cantrip_mechanics::{attach, tokens, triggers};
use cantrip_oracle as oracle;

/// "When this Aura enters, create a 1/1 white Warrior creature token with vigilance. Enchanted creature gets +1/+1 and has first strike. (It deals combat damage before creatures without first strike.)" (CR 608.2).
fn create_1_1_white_warrior_enchant(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) && asker.controller == q.controller(id)))
    )
}

/// "When this Aura enters, create a 1/1 white Warrior creature token with vigilance." (CR 608.2, 111.1).
fn create_1_1_white_warrior(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_0 = Vec::new();
    let def_0_0 = g.card_id(&tokens::WARRIOR_1_1_WHITE);
    tokens_0.extend(std::iter::repeat((def_0_0, item.controller)).take(1));
    let words_0 = g.words(item.object);
    g.create_tokens_worded(&tokens_0, false, None, words_0)?;
    Ok(())
}

/// "Enchanted creature gets +1/+1 and has first strike." (CR 604.1).
fn enchanted_creature_gets_1_1_apply_1(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.keywords = chars.keywords.with(Keyword::FirstStrike);
}

/// "Enchanted creature gets +1/+1 and has first strike." (CR 604.1).
fn enchanted_creature_gets_1_1_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    let (p, t) = (1i16, 1i16);
    chars.power = chars.power.map(|v| v.saturating_add(p));
    chars.toughness = chars.toughness.map(|v| v.saturating_add(t));
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            create_1_1_white_warrior,
            "When this Aura enters, create a 1/1 white Warrior creature token with vigilance.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            enchanted_creature_gets_1_1_apply_1,
            "Enchanted creature gets +1/+1 and has first strike.",
        ),
        static_ability(
                Layer::PtModify,
                attach::attached_object,
                enchanted_creature_gets_1_1_apply_0,
                "Enchanted creature gets +1/+1 and has first strike.",
            )
            .continues(),
    ],
    enchant: Some(target(create_1_1_white_warrior_enchant)),
    ..oracle::CARTOUCHE_OF_SOLIDARITY
};

pub static EXTRA: &[&CardDef] = &[&tokens::WARRIOR_1_1_WHITE];
