use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Whenever you cast a multicolored spell" (CR 603.2): a spell its controller casts that is two or more colors as it is
/// on the stack (CR 105.2b, 601.2i).
fn multicolored_spell_you_cast(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && q.chars(*spell).colors.0.count_ones() >= 2 => Fired::PLAIN,
        _ => None,
    }
}

/// "Create a 4/4 red and white Golem artifact creature token." Its colors and creature type are the words of the text
/// that makes it (CR 612.4).
fn make_golem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &GOLEM, 1)?;
    Ok(())
}

/// The 4/4 red and white Golem artifact creature token.
pub static GOLEM: CardDef = CardDef {
    name: "Golem Token",
    token_name: Some(&[Subtype::Golem]),
    has_cost: false,
    colors: Some(ColorSet(ColorSet::of(Color::Red).0 | ColorSet::of(Color::White).0)),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Golem]),
    power: Some(4),
    toughness: Some(4),
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    // "Hexproof from monocolored" (CR 702.11d).
    hexproof_from: HexproofFrom::MONOCOLORED,
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, multicolored_spell_you_cast),
        make_golem,
        "Whenever you cast a multicolored spell, create a 4/4 red and white Golem artifact creature token.",
    )],
    ..oracle::GENERAL_FERROUS_ROKIRIC
};

pub static EXTRA: &[&CardDef] = &[&GOLEM];
