use cantrip_api::*;
use cantrip_mechanics::{Animation, animate, targets, tokens, triggers::NONCREATURE_SPELL_YOU_CAST};
use cantrip_oracle as oracle;

/// "Until end of turn, target artifact token you control becomes a 3/1 Construct artifact creature with flying" (CR 613.1d, 613.4b):
/// Artifact and Creature are added to its types, Construct (read through the Mechanist's words, CR 612) replaces any creature types
/// it had, since the text does not say it is still what it was (CR 205.1b), its base power and toughness are set to 3/1 over any
/// set before (layer 7b) and flying is granted (6). Its abilities, other types and other subtypes stay (the rulings), so a Clue is
/// still a Clue. The token has to have been under its controller's control since the turn began to attack (CR 302.6), however
/// long it has been a creature.
fn becomes_construct(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(token) = targets::object(g, item, 0) else { return Ok(()) };
    let animation = Animation {
        power: 3,
        toughness: 1,
        colors: None,
        types: TypeSet::of(&[CardType::Artifact]),
        subtypes: &[g.words(item.object).subtype(Subtype::Construct)],
        all_creature_types: false,
        replaces_creature_types: true,
        keywords: Keywords::of(&[Keyword::Flying]),
        ability: None,
    };
    animate(g, item.object, token, &animation, Duration::EndOfTurn);
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(NONCREATURE_SPELL_YOU_CAST, |g, item| tokens::create(g, item.controller, &tokens::CLUE, 1).map(|_| ()), "Whenever you cast a noncreature spell, create a Clue token. (It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")"),
        ability(TAP, becomes_construct, "{T}: Until end of turn, target artifact token you control becomes a 3/1 Construct artifact creature with flying.")
            .targets(&[targets::ARTIFACT_TOKEN_YOU_CONTROL]),
    ],
    ..oracle::THE_MECHANIST_AERIAL_ARTISAN
};
