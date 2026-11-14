use cantrip_api::*;
use cantrip_mechanics::effects::{ask_may, two_plus_one_counters_on_this};
use cantrip_mechanics::{Animation, animate};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Animate", "have it become a 0/0 Elemental creature until end of turn?", "It's still a land", "Become a creature", "Don't");

/// "Put two +1/+1 counters on this land. Then you may have it become a 0/0 Elemental creature until end of turn. It's still a
/// land." (CR 122.1, 613.1d, 613.4b, 608.2c): the counters first, whether or not it is then animated; the choice is made as the
/// ability resolves, and animating changes the land's base power and toughness to 0/0 so the counters make it as big as they
/// are many. With no counters put on it (Solemnity), a land that becomes the 0/0 creature dies as a state-based action (CR 704.5f).
/// The text names no color, so the land stays colorless.
fn grow_and_maybe_animate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    two_plus_one_counters_on_this(g, item)?;
    g.end_sentence();
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        let animation = Animation {
            power: 0,
            toughness: 0,
            colors: None,
            types: TypeSet::NONE,
            subtypes: &[g.words(item.object).subtype(Subtype::Elemental)],
            all_creature_types: false,
            replaces_creature_types: false,
            keywords: Keywords::NONE,
            ability: None,
        };
        animate(g, item.object, item.object, &animation, Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
            cost("{4}"),
            grow_and_maybe_animate,
            "{4}: Put two +1/+1 counters on this land. Then you may have it become a 0/0 Elemental creature until end of turn. It's still a land.",
        ),
    ],
    ..oracle::CRAWLING_BARRENS
};
