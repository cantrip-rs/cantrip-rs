use cantrip_api::*;
use cantrip_mechanics::{choose, effects, subtype_cards_in_graveyard, triggers};
use cantrip_oracle as oracle;

/// "Mill three cards. Then if there is an Elf card in your graveyard, you gain 2 life." The cards are milled first, so a milled Elf
/// counts (CR 608.2c); the Elf is the word of this card's text (CR 612).
fn mill_then_gain_life_with_an_elf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 3);
    let elf = g.words(item.object).subtype(Subtype::Elf);
    if subtype_cards_in_graveyard(g, item.controller, elf) > 0 {
        effects::gain_life(g, item, 2)?;
    }
    Ok(())
}

/// "Mill three cards, then you may exile an Elf card from your graveyard. If you do, each opponent loses 2 life." Milled Elves
/// can be exiled; none to exile, or none chosen, ends it.
fn mill_then_may_exile_an_elf_for_life_loss(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    g.mill(you, 3);
    let elf = g.words(item.object).subtype(Subtype::Elf);
    let elves: Vec<ObjectId> = g.state().zone(Zone::graveyard(you)).iter().copied().filter(|&card| g.chars(card).has_subtype(elf)).collect();
    let chosen = choose::cards(g, you, item.object, Purpose::Exile, elves, 1, true)?;
    if let Some(&card) = chosen.first() {
        g.exile(card);
        g.each_opponent_loses_life(you, 2);
    }
    Ok(())
}

/// "At the beginning of your first main phase, you may pay {B}. If you do, transform Trystan."
fn pay_black(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{B}"))
}

/// "At the beginning of your first main phase, you may pay {G}. If you do, transform Trystan."
fn pay_green(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{G}"))
}

/// Deathtouch is a core keyword.
pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_TRANSFORMS,
            mill_then_may_exile_an_elf_for_life_loss,
            "Whenever this creature transforms into Trystan, Penitent Culler, mill three cards, then you may exile an Elf card from your graveyard. If you do, each opponent loses 2 life.",
        ),
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_green, "At the beginning of your first main phase, you may pay {G}. If you do, transform Trystan."),
    ],
    ..oracle::TRYSTAN_PENITENT_CULLER
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS_OR_TRANSFORMS,
            mill_then_gain_life_with_an_elf,
            "Whenever this creature enters or transforms into Trystan, Callous Cultivator, mill three cards. Then if there is an Elf card in your graveyard, you gain 2 life.",
        ),
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_black, "At the beginning of your first main phase, you may pay {B}. If you do, transform Trystan."),
    ],
    back: Some(&BACK),
    ..oracle::TRYSTAN_CALLOUS_CULTIVATOR
};
