use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static BIRD: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet(8)),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..token!(Bird)
};

/// "When this creature dies, create a 4/4 red Bird creature token with flying at the beginning of the next end step." (CR 608.2).
fn delayed_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_2 = Vec::new();
    let def_2_0 = g.card_id(&BIRD);
    tokens_2.extend(std::iter::repeat((def_2_0, item.controller)).take(1));
    let words_2 = g.words(item.object);
    g.create_tokens_worded(&tokens_2, false, None, words_2)?;
    Ok(())
}

/// "When this creature dies, create a 4/4 red Bird creature token with flying at the beginning of the next end step." (CR 608.2).
fn create_4_4_red_bird(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.delayed_trigger(
        DelayedWhen::Step {
            step: Step::End,
            on_turn_of: None,
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_DIES,
            create_4_4_red_bird,
            "When this creature dies, create a 4/4 red Bird creature token with flying at the beginning of the next end step.",
        ),
    ],
    delayed: &[
        delayed_trigger(
            delayed_1,
            "When this creature dies, create a 4/4 red Bird creature token with flying at the beginning of the next end step.",
        ),
    ],
    ..oracle::RUKH_EGG
};

pub static EXTRA: &[&CardDef] = &[&BIRD];
