use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{choose, copy, delayed, exile_and_return, exile_and_return_transformed, library, targets};
use cantrip_oracle as oracle;

/// "Mill five cards. Put up to one enchantment card milled this way into your hand" (CR 701.17, 701.17c): the enchantment cards
/// are found where the milled cards went; nobody is asked when none was milled, and choosing none leaves them all.
fn mill_five(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::mill_then_may_take_into_hand(g, item.controller, item.object, 5, "enchantment", |c| c.is(CardType::Enchantment))?;
    Ok(())
}

/// "Exile Terra, then return it to the battlefield transformed under its owner's control" (CR 712.14a): the new object is Esper
/// Terra, a Saga that enters with a lore counter (CR 714.3a). Nothing returns if Terra has left the battlefield already (CR 400.7).
fn transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

/// "Target nonlegendary enchantment you control" (CR 115.1, 205.4a), judged now: Esper Terra herself is legendary.
fn nonlegendary_enchantment_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && q.controller(id) == asker.controller && {
        let c = q.chars(id);
        c.is(CardType::Enchantment) && !c.supertypes.legendary
    })
}

/// I, II, III: "Create a token that's a copy of target nonlegendary enchantment you control. It gains haste. If it's a Saga, put up
/// to three lore counters on it. Sacrifice it at the beginning of your next end step." (CR 707.2, 714.2b, 603.7c): a Saga copy
/// enters with its own lore counter (CR 714.3a), and the counters put on it after trigger every chapter they carry it to at once.
/// The delayed sacrifice is about exactly the tokens made, and is "your" next end step: this turn's, as the chapter triggers
/// after the draw step.
fn copy_an_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(enchantment) = targets::object(g, item, 0) else { return Ok(()) };
    let values = g.copiable_values(enchantment);
    let tokens = copy::create_token_copy_with_haste(g, item, values, Duration::Indefinite)?;
    for &token in &tokens {
        if g.is_on_battlefield(token) && g.chars(token).has_subtype(Subtype::Saga) {
            let counters = choose::number(g, item.controller, item.object, 0, 3, "How many lore counters to put on it")?;
            g.add_counters(token, CounterKind::Lore, counters as i32);
        }
    }
    g.end_sentence();
    delayed::at_your_next_end_step_about(g, item, 0, &tokens);
    Ok(())
}

/// IV: "Add {W}{W}, {U}{U}, {B}{B}, {R}{R}, and {G}{G}. Exile Esper Terra, then return it to the battlefield (front face up)"
/// (CR 701.13, 712.14a): the mana first (it stays through the phase), then the exile and return as a new object; nothing returns
/// if she has already left the battlefield.
fn add_mana_and_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for mana in [Mana::W, Mana::U, Mana::B, Mana::R, Mana::G] {
        g.add_mana(item.controller, mana, 2);
    }
    exile_and_return(g, &[item.object])
}

/// Flying is a core keyword read from its characteristics.
pub static BACK: CardDef = CardDef {
    abilities: &[
        chapter(1, copy_an_enchantment, "I \u{2014} Create a token that's a copy of target nonlegendary enchantment you control. It gains haste. If it's a Saga, put up to three lore counters on it. Sacrifice it at the beginning of your next end step.")
            .targets(&[target(nonlegendary_enchantment_you_control)]),
        chapter(2, copy_an_enchantment, "II \u{2014} Create a token that's a copy of target nonlegendary enchantment you control. It gains haste. If it's a Saga, put up to three lore counters on it. Sacrifice it at the beginning of your next end step.")
            .targets(&[target(nonlegendary_enchantment_you_control)]),
        chapter(3, copy_an_enchantment, "III \u{2014} Create a token that's a copy of target nonlegendary enchantment you control. It gains haste. If it's a Saga, put up to three lore counters on it. Sacrifice it at the beginning of your next end step.")
            .targets(&[target(nonlegendary_enchantment_you_control)]),
        chapter(4, add_mana_and_return, "IV \u{2014} Add {W}{W}, {U}{U}, {B}{B}, {R}{R}, and {G}{G}. Exile Esper Terra, then return it to the battlefield (front face up)."),
    ],
    delayed: &[delayed_trigger(delayed::sacrifice_created, "Sacrifice it at the beginning of your next end step.")],
    ..oracle::ESPER_TERRA
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, mill_five, "When Terra enters, mill five cards. Put up to one enchantment card milled this way into your hand."),
        ability(
            cost("{4}{R}{G}").and(TAP),
            transform,
            "Trance \u{2014} {4}{R}{G}, {T}: Exile Terra, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::TERRA_MAGICAL_ADEPT
};
