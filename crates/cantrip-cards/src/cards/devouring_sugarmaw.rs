use cantrip_api::*;
use cantrip_mechanics::{effects, select, tokens, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, you may sacrifice an artifact, enchantment, or token. If you don't, tap this creature." (CR 701.21a,
/// 608.2c-d): the sacrifice is chosen as the ability resolves and not paid, so a player with nothing to sacrifice is not asked; declining
/// and having nothing are the same, and tap this creature (a tapped one stays tapped).
fn sacrifice_or_tap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = Selection::of(select::up_to(1), &[select::ARTIFACT_ENCHANTMENT_OR_TOKEN]).called("artifact, enchantment, or token");
    let sacrificed = effects::may_sacrifice(g, item.controller, item.object, rule)?;
    g.end_sentence();
    if sacrificed.is_empty() {
        g.tap(item.object);
    }
    Ok(())
}

/// "Create a 1/1 white Human creature token and a Food token." (CR 111.10b): both are created together, as one event.
fn create_a_human_and_a_food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let (human, food) = (g.card_id(&tokens::HUMAN), g.card_id(&tokens::FOOD));
    g.create_tokens_worded(&[(human, you), (food, you)], false, None, Words::NONE)?;
    Ok(())
}

pub static HAVE_FOR_DINNER: CardDef = CardDef { spell: spell(&[], create_a_human_and_a_food), ..oracle::HAVE_FOR_DINNER };

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        sacrifice_or_tap,
        "At the beginning of your upkeep, you may sacrifice an artifact, enchantment, or token. If you don't, tap this creature.",
    )],
    adventure: Some(&HAVE_FOR_DINNER),
    ..oracle::DEVOURING_SUGARMAW
};

pub static EXTRA: &[&CardDef] = &[&HAVE_FOR_DINNER, &tokens::HUMAN, &tokens::FOOD];
