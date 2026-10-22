use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::{noncreature_spell_you_cast, NONCREATURE_SPELL_YOU_CAST};
use cantrip_oracle as oracle;

const ASK_1: Question = Question::new("Animate", "become an artifact creature until end of turn?", "", "Become a creature", "Don't");
const ASK_2: Question = Question::new("SacrificeForTokens", "sacrifice it?", "If you do, create four 4/4 colorless Construct artifact creature tokens with flying and haste", "Sacrifice it", "Keep it");

/// A 4/4 colorless Construct artifact creature token with flying and haste. Other Constructs of the same name are
/// different definitions (Urza's, Retrofitter Foundry's).
pub static CONSTRUCT: CardDef = CardDef {
    name: "Construct Token",
    token_name: Some(&[Subtype::Construct]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Construct]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Haste]),
    shares_name: true,
    text: "Flying, haste",
    ..CardDef::BLANK
};

/// You may have The Fantasticar become an artifact creature until end of turn.
fn animate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !ask_may(g, item.controller, item.object, Purpose::Ask(&ASK_1))? {
        return Ok(());
    }
    if is_live(g, item.object) && g.is_on_battlefield(item.object) {
        g.add_effect(item.object, item.object, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::EndOfTurn);
    }
    Ok(())
}

/// "Whenever you cast your fourth noncreature spell each turn" (CR 601.2i, 603.2): counted in the event history, which
/// holds this cast and the ones before it whether they resolved or were countered. Abilities are not spells.
fn fourth_noncreature_spell(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    noncreature_spell_you_cast(q, me, event)?;
    let you = me.controller;
    let noncreature = q.history().casts_by(you).filter(|c| !c.is(CardType::Creature)).count();
    if noncreature == 4 { Fired::PLAIN } else { None }
}

/// You may sacrifice The Fantasticar. If you do, create four 4/4 colorless Construct artifact creature tokens with
/// flying and haste.
fn sacrifice_and_create(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let me = item.object;
    let here = |g: &dyn Game| is_live(g, me) && g.is_on_battlefield(me) && g.obj(me).controller == you;
    if !here(g) {
        return Ok(());
    }
    if !ask_may(g, you, me, Purpose::Ask(&ASK_2))? {
        return Ok(());
    }
    if !here(g) {
        return Ok(());
    }
    if g.sacrifice(&[me]).is_empty() {
        return Ok(());
    }
    tokens::create(g, you, &CONSTRUCT, 4)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(NONCREATURE_SPELL_YOU_CAST, animate, "Whenever you cast a noncreature spell, you may have The Fantasticar become an artifact creature until end of turn."),
        triggered(when_asked(EventMask::SPELL_CAST, fourth_noncreature_spell), sacrifice_and_create, "Whenever you cast your fourth noncreature spell each turn, you may sacrifice The Fantasticar. If you do, create four 4/4 colorless Construct artifact creature tokens with flying and haste."),
    ],
    ..oracle::THE_FANTASTICAR
};

pub static EXTRA: &[&CardDef] = &[&CONSTRUCT];
