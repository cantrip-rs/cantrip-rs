use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::statics::creature_attacking_triggers_again;
use cantrip_mechanics::triggers::your_upkeep;
use cantrip_mechanics::tokens::{self, GOBLIN};
use cantrip_oracle as oracle;

const MARDU: u8 = 0;
const JESKAI: u8 = 1;

/// "As this enchantment enters, choose Mardu or Jeskai" (CR 614.12a, 607.2f).
fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    choose::option_as_enters(g, entering, &CARD, 0..2)
}

/// "• Mardu — If a creature attacking causes a triggered ability of a permanent you control to trigger, that ability
/// triggers an additional time" (CR 603.2d, 508.2a): the rule of Isshin, Two Heavens as One, only while Mardu is the word
/// chosen for this enchantment.
fn mardu_triggers_again(q: &dyn Query, me: &Asker, event: &Event, triggering: &Triggering) -> bool {
    choose::chosen_option(q, me.source, &CARD) == Some(MARDU)
        && creature_attacking_triggers_again(q, me, event, triggering)
}

/// "• Jeskai — At the beginning of your upkeep" (CR 503.1, 603.2): your own upkeep, and only while Jeskai was chosen.
fn jeskai_upkeep(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    (choose::chosen_option(q, me.source, &CARD) == Some(JESKAI)).then(|| your_upkeep(q, me, event)).flatten()
}

/// "Create a 1/1 red Goblin creature token. It gains lifelink and haste until end of turn." The token arrives first and
/// then gains the two keywords as an effect of its own, until the cleanup step (CR 611.2a).
fn create_goblin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let keywords = Keywords::of(&[Keyword::Lifelink, Keyword::Haste]);
    let made = tokens::create(g, item.controller, &GOBLIN, 1)?;
    instruction(g, |g| {
        for token in made {
            g.add_effect(item.object, token, EffectKind::AddKeywords(keywords), Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&GOBLIN];

pub static CARD: CardDef = CardDef {
    enters: Some(enters),
    labels: &["Mardu", "Jeskai"],
    abilities: &[triggered(
        when_asked(EventMask::STEP_BEGAN, jeskai_upkeep),
        create_goblin,
        "Jeskai \u{2014} At the beginning of your upkeep, create a 1/1 red Goblin creature token. It gains lifelink and haste until end of turn.",
    )],
    additional_triggers: &[mardu_triggers_again],
    ..oracle::WINDCRAG_SIEGE
};
