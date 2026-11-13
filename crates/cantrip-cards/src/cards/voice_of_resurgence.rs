use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

/// The green and white Elemental creature token it makes (CR 111.4), whose size is the number of creatures its controller has.
pub static ELEMENTAL: CardDef = CardDef {
    // White (bit 0) and green (bit 4).
    colors: Some(ColorSet(0b10001)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    statics: &[characteristic_defining(Layer::PtDefine, lands::power_and_toughness_are_creatures_you_control, "This token's power and toughness are each equal to the number of creatures you control.").lost_pt(0, 0)],
    ..token!(Elemental)
};

/// "Whenever an opponent casts a spell during your turn" (CR 601.2i, 603.2): a spell cast (CR 707.10: not a copy) by a
/// player other than the ability's controller, while its controller is the active player.
fn opponent_casts_a_spell_during_your_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { player, .. } if *player != me.controller && q.state().turn.active == me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "Create a green and white Elemental creature token with ..." (CR 111.2: under the ability's controller).
fn create_elemental(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ELEMENTAL, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when_asked(EventMask::SPELL_CAST, opponent_casts_a_spell_during_your_turn), create_elemental, "Whenever an opponent casts a spell during your turn, create a green and white Elemental creature token with \"This token's power and toughness are each equal to the number of creatures you control.\""),
        triggered(THIS_DIES, create_elemental, "When this creature dies, create a green and white Elemental creature token with \"This token's power and toughness are each equal to the number of creatures you control.\""),
    ],
    ..oracle::VOICE_OF_RESURGENCE
};

pub static EXTRA: &[&CardDef] = &[&ELEMENTAL];
