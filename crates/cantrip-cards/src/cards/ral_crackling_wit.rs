use cantrip_api::*;
use cantrip_mechanics::{discard_chosen, effects, statics, storm, tokens, triggers};
use cantrip_oracle as oracle;

/// +1: "Create a 1/1 blue and red Otter creature token with prowess." (CR 111.1)
fn otter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::OTTER, 1)?;
    Ok(())
}

/// -3: "Draw three cards, then discard two cards." (CR 121.1, 701.9)
fn draw_three_discard_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 3);
    discard_chosen(g, item.controller, item.object, 2)?;
    Ok(())
}

/// -10: "Draw three cards. You get an emblem with ..." (CR 114.2)
fn draw_three_and_emblem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 3);
    tokens::create_emblem(g, item.controller, &EMBLEM);
    Ok(())
}

// --- The emblem ---------------------------------------------------------------------------------------------------

pub static EMBLEM: CardDef = CardDef {
    name: "Ral, Crackling Wit emblem",
    has_cost: false,
    delayed: &[storm::trigger("Storm (When you cast this spell, copy it for each spell cast before it this turn.)")],
    statics: &[static_ability(Layer::Ability, statics::instant_or_sorcery_spell_you_cast, statics::grant_delayed_ability, "Instant and sorcery spells you cast have storm.")
        .everywhere()],
    text: "Instant and sorcery spells you cast have storm.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::NONCREATURE_SPELL_YOU_CAST, effects::put_loyalty_counter_on_this, "Whenever you cast a noncreature spell, put a loyalty counter on Ral."),
        ability(loyalty(1), otter, "+1: Create a 1/1 blue and red Otter creature token with prowess."),
        ability(loyalty(-3), draw_three_discard_two, "\u{2212}3: Draw three cards, then discard two cards."),
        ability(loyalty(-10), draw_three_and_emblem, "\u{2212}10: Draw three cards. You get an emblem with \"Instant and sorcery spells you cast have storm.\" (Whenever you cast an instant or sorcery spell, copy it for each spell cast before it this turn.)"),
    ],
    ..oracle::RAL_CRACKLING_WIT
};

pub static EXTRA: &[&CardDef] = &[&tokens::OTTER, &EMBLEM];
