use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

const LEGENDARY: Supertypes = Supertypes { basic: false, legendary: true, snow: false, world: false };

/// I: "Create a 2/1 green Spider creature token with reach." (CR 111.1)
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SPIDER, 1)?;
    Ok(())
}

/// II: "Put a +1/+1 counter on target creature you control. It becomes a legendary Spider Hero in addition to its other types."
/// (CR 122.1a, 205.1b, 205.4a, 613.1d): the counter goes on first; then the creature gains the legendary supertype and the
/// Spider and Hero subtypes in layer 4, for as long as it stays on the battlefield (CR 611.2a, 400.7). Nothing happens if the
/// target is no longer legal as the chapter resolves (CR 608.2b). "Spider" and "Hero" are words of the Saga's text (CR 612).
fn chapter_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    let spider = g.words(item.object).subtype(Subtype::Spider);
    let hero = g.words(item.object).subtype(Subtype::Hero);
    instruction(g, |g| {
        g.add_effect(item.object, creature, EffectKind::AddSupertypes(LEGENDARY), Duration::Indefinite);
        g.add_effect(item.object, creature, EffectKind::AddSubtype(spider), Duration::Indefinite);
        g.add_effect(item.object, creature, EffectKind::AddSubtype(hero), Duration::Indefinite);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} Create a 2/1 green Spider creature token with reach."),
        chapter(
            2,
            chapter_two,
            "II \u{2014} Put a +1/+1 counter on target creature you control. It becomes a legendary Spider Hero in addition to its other types.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        chapter(3, effects::target_gains_double_strike, "III \u{2014} Target creature you control gains double strike until end of turn.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    ..oracle::ORIGIN_OF_SPIDER_MAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::SPIDER];
