use cantrip_api::*;
use cantrip_mechanics::{Animation, animate, effects, history};
use cantrip_oracle as oracle;

/// "At the beginning of each combat" (CR 507.1): the beginning of combat step of any player's turn.
fn each_combat(_: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::BeginCombat } => Fired::PLAIN,
        _ => None,
    }
}

/// "... if you've cast two or more spells this turn" (CR 603.4): asked as the combat begins and again as the ability
/// resolves, of the spells its controller has cast this turn, on whosever turn it is.
fn you_cast_two_or_more_spells_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    history::cast_two_or_more_spells_this_turn(q.history(), asker.controller)
}

/// "This enchantment becomes a 3/3 Monk creature with haste in addition to its other types until end of turn" (CR 205.1b,
/// 613.1d): the Creature type and the creature type Monk (a word of its text, read through its words, CR 612) are added
/// to its types, haste is granted and its base power and toughness set to 3/3, so it keeps its color; it is an enchantment
/// still. Haste lets it attack even if it came under its controller's control this turn (CR 302.6, 702.10b).
fn becomes_monk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let words = g.words(item.object);
    let animation = Animation {
        power: 3,
        toughness: 3,
        colors: None,
        types: TypeSet::NONE,
        subtypes: &[words.subtype(Subtype::Monk)],
        all_creature_types: false,
        replaces_creature_types: false,
        keywords: Keywords::of(&[Keyword::Haste]),
        ability: None,
    };
    animate(g, item.object, item.object, &animation, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::STEP_BEGAN, each_combat),
            becomes_monk,
            "At the beginning of each combat, if you've cast two or more spells this turn, this enchantment becomes a 3/3 Monk creature with haste in addition to its other types until end of turn.",
        )
        .condition(you_cast_two_or_more_spells_this_turn),
        ability(
            cost("{1}{R}").and(DISCARD_HAND).and(SACRIFICE_THIS),
            |g, item| effects::draw(g, item, 2),
            "{1}{R}, Discard your hand, Sacrifice this enchantment: Draw two cards.",
        ),
    ],
    ..oracle::REVERBERATING_SUMMONS
};
