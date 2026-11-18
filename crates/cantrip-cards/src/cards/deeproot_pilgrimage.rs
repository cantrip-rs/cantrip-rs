use cantrip_api::*;
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

/// The 1/1 blue Merfolk creature token with hexproof.
pub static MERFOLK: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Hexproof]),
    ..token!(Merfolk)
};

/// "Whenever one or more nontoken Merfolk you control become tapped" (CR 701.26a, 603.2c, 111.7): a permanent that is a Merfolk now
/// (read through the words of this enchantment's text, CR 612.2), is not a token, and is controlled by this enchantment's
/// controller, whatever tapped it. Those tapped together are the batch's subjects, so they trigger it once.
fn nontoken_merfolk_you_control_become_tapped(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::Tapped { permanent } = event else { return None };
    let id = *permanent;
    let merfolk = q.chars(id).has_subtype(lands::word(q, me.source, lands::sub(Subtype::Merfolk)));
    (merfolk && !q.obj(id).token && q.controller(id) == me.controller).then(|| Fired::about(Target::Object(id))).flatten()
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::TAPPED, nontoken_merfolk_you_control_become_tapped),
        |g, item| tokens::create(g, item.controller, &MERFOLK, 1).map(|_| ()),
        "Whenever one or more nontoken Merfolk you control become tapped, create a 1/1 blue Merfolk creature token with hexproof.",
    )
    .once_per_batch()],
    ..oracle::DEEPROOT_PILGRIMAGE
};

pub static EXTRA: &[&CardDef] = &[&MERFOLK];
