use cantrip_api::*;
use cantrip_mechanics::search_library;
use cantrip_mechanics::{lands, mana};
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// +1: Put three +1/+1 counters on up to one target noncreature land you control. Untap it. It becomes a 0/0 Elemental creature
/// with vigilance and haste that's still a land. It states no duration (CR 611.2a), so the land stays a creature: the type added
/// rather than set (CR 205.1b), and the base 0/0 in layer 7b under the counters of 7c.
fn animate_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.target_count == 0 || !g.target_legal(item, 0) {
        return Ok(());
    }
    let Target::Object(land) = item.target(0) else { return Ok(()) };
    g.add_counters(land, CounterKind::PlusOne, 3);
    g.end_sentence();
    g.untap(land);
    g.end_sentence();
    let elemental = g.words(item.object).subtype(Subtype::Elemental);
    instruction(g, |g| {
        g.add_effect(item.object, land, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::Indefinite);
        g.add_effect(item.object, land, EffectKind::AddSubtype(elemental), Duration::Indefinite);
        g.add_effect(item.object, land, EffectKind::AddKeywords(Keywords::of(&[Keyword::Vigilance, Keyword::Haste])), Duration::Indefinite);
        g.add_effect(item.object, land, EffectKind::SetPt(0, 0), Duration::Indefinite);
    });
    Ok(())
}

/// -8: You get an emblem with "Lands you control have indestructible." Search your library for any number of Forest cards, put
/// them onto the battlefield tapped, then shuffle.
fn emblem_and_forests(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_emblem(g, item.controller, &EMBLEM);
    let you = item.controller;
    let forest = g.words(item.object).subtype(Subtype::Forest);
    let found = search_library(g, you, you, item.object, SearchDestination::BattlefieldTapped, cantrip_mechanics::ANY_NUMBER, true, |q, c| q.chars(c).has_subtype(forest))?;
    let lands: Vec<(ObjectId, PlayerId, bool)> = found.into_iter().map(|c| (c, you, true)).collect();
    g.put_all_onto_battlefield_as(&lands)?;
    g.shuffle(you);
    Ok(())
}

// --- The emblem ---------------------------------------------------------------------------------------------------

fn indestructible(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = Keywords(chars.keywords.0 | Keywords::of(&[Keyword::Indestructible]).0);
}

pub static EMBLEM: CardDef = CardDef {
    name: "Nissa, Who Shakes the World emblem",
    has_cost: false,
    statics: &[static_ability(Layer::Ability, cantrip_mechanics::statics::your_lands, indestructible, "Lands you control have indestructible.")],
    text: "Lands you control have indestructible.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::TAPPED, mana::you_tap_a_land_type_for_mana::<{ lands::sub(Subtype::Forest) }>),
            mana::that_player_adds_an_additional::<{ Mana::G as u8 }>,
            "Whenever you tap a Forest for mana, add an additional {G}.",
        )
            .mana_ability(TriggeredMana::One(Mana::G)),
        ability(loyalty(1), animate_land, "+1: Put three +1/+1 counters on up to one target noncreature land you control. Untap it. It becomes a 0/0 Elemental creature with vigilance and haste that's still a land.")
            .targets(&[up_to_one(targets::noncreature_land_you_control)]),
        ability(loyalty(-8), emblem_and_forests, "\u{2212}8: You get an emblem with \"Lands you control have indestructible.\" Search your library for any number of Forest cards, put them onto the battlefield tapped, then shuffle."),
    ],
    ..oracle::NISSA_WHO_SHAKES_THE_WORLD
};

pub static EXTRA: &[&CardDef] = &[&EMBLEM];
