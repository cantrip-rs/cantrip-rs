use cantrip_api::*;
use cantrip_mechanics::library::{Find, search_to_battlefield};
use cantrip_mechanics::tokens::{self, CONSTRUCT};
use cantrip_oracle as oracle;

/// I: "This Saga gains '{T}: Add {C}.'", for as long as it stays (CR 611.2a).
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0)), Duration::Indefinite);
    Ok(())
}

/// II: "This Saga gains '{2}, {T}: Create a 0/0 colorless Construct ...'", independently of its lore counters.
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 1)), Duration::Indefinite);
    Ok(())
}

fn make_construct(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &CONSTRUCT, 1)?;
    Ok(())
}

/// An artifact card whose mana cost is exactly {0} or {1}: no colored, colorless or hybrid symbol, no X.
fn cheap_artifact(q: &dyn Query, id: ObjectId) -> bool {
    let card = q.card(q.obj(id).card);
    let c = &card.cost;
    card.types.has(CardType::Artifact)
        && card.has_cost
        && c.generic <= 1
        && c.symbols == [0; 6]
        && c.hybrid == [None; 6]
        && c.phyrexian == [0; 5]
        && c.x == 0
}

/// III: search for an artifact card with mana cost {0} or {1}, put it onto the battlefield, then shuffle.
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    search_to_battlefield(g, item.controller, item.object, Find::ONE, false, cheap_artifact)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} This Saga gains \"{T}: Add {C}.\""),
        chapter(2, chapter_two, "II \u{2014} This Saga gains \"{2}, {T}: Create a 0/0 colorless Construct artifact creature token with 'This token gets +1/+1 for each artifact you control.'\""),
        chapter(3, chapter_three, "III \u{2014} Search your library for an artifact card with mana cost {0} or {1}, put it onto the battlefield, then shuffle."),
    ],
    delayed: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{2}, {T}"), make_construct, "{2}, {T}: Create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\""),
    ],
    ..oracle::URZAS_SAGA
};

pub static EXTRA: &[&CardDef] = &[&CONSTRUCT];
