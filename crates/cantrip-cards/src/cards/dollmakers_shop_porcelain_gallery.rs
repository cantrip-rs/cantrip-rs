//! Dollmaker's Shop // Porcelain Gallery, a Room (CR 709.5): each door is only what its text does; the Room has the
//! abilities of its unlocked doors (`layers::doors`, `CardTable::texts`).

use cantrip_api::*;
use cantrip_mechanics::{lands, statics, tokens};
use cantrip_oracle as oracle;

/// The 1/1 white Toy artifact creature token Dollmaker's Shop creates (CR 111.4, 205.3m); no other printed card makes it.
pub static TOY: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..token!(Toy)
};

/// "Whenever one or more non-Toy creatures you control attack a player" (CR 508.1, 603.2c): a creature its controller
/// controls that is not a Toy (the type read through the words of the Room's text, CR 612.2), declared as an attacker of a
/// player rather than a planeswalker or battle. Each player attacked is a batch of its own (`batch_by_data` on the
/// defender), so the ability triggers once for each player those creatures attack, whatever their number.
fn non_toy_creatures_you_control_attack_a_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, defender: Target::Player(player), .. }
            if q.controller(*attacker) == me.controller && !q.chars(*attacker).has_subtype(q.words(me.source).subtype(Subtype::Toy)) =>
        {
            Some(Fired { subject: Target::Object(*attacker), data: [0, player.0 as u32] })
        }
        _ => None,
    }
}

fn create_a_toy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &TOY, 1)?;
    Ok(())
}

pub static DOLLMAKERS_SHOP: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACKS, non_toy_creatures_you_control_attack_a_player),
        create_a_toy,
        "Whenever one or more non-Toy creatures you control attack a player, create a 1/1 white Toy artifact creature token.",
    )
    .once_per_batch()
    .batch_by_data()],
    ..oracle::DOLLMAKERS_SHOP
};

/// Porcelain Gallery: "Creatures you control have base power and toughness each equal to the number of creatures you
/// control" (CR 613.4b, layer 7b), while this door is unlocked. The creatures are counted as the earlier layers left them,
/// each affected creature among them; counters and other modifying effects (layer 7c) then apply on top.
pub static PORCELAIN_GALLERY: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtSet,
        statics::your_creatures,
        lands::power_and_toughness_are_creatures_you_control,
        "Creatures you control have base power and toughness each equal to the number of creatures you control.",
    )],
    ..oracle::PORCELAIN_GALLERY
};

pub static CARD: CardDef = CardDef { split: Some((&DOLLMAKERS_SHOP, &PORCELAIN_GALLERY)), ..oracle::DOLLMAKERS_SHOP_PORCELAIN_GALLERY };

pub static EXTRA: &[&CardDef] = &[&DOLLMAKERS_SHOP, &PORCELAIN_GALLERY, &TOY];
