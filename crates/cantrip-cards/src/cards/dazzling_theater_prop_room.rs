//! Dazzling Theater // Prop Room, a Room (CR 709.5): each door is only what its text does; the Room has the abilities of
//! its unlocked doors (`layers::doors`, `CardTable::texts`).

use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// Dazzling Theater: "Creature spells you cast have convoke" while this door is unlocked.
pub static DAZZLING_THEATER: CardDef = CardDef {
    grants_pay_with: &[statics::creature_spells_you_cast_have_convoke],
    ..oracle::DAZZLING_THEATER
};

/// "Untap each creature you control during each other player's untap step" (CR 502.3, 611.3a): every creature the Room's
/// controller controls untaps in each opponent's untap step, as that player's permanents untap; their other permanents
/// don't.
fn creatures_you_control(q: &dyn Query, asker: &Asker, object: ObjectId) -> bool {
    q.chars(object).is(CardType::Creature) && q.obj(object).controller == asker.controller
}

pub static PROP_ROOM: CardDef = CardDef { untap_in_others_untap_steps: &[creatures_you_control], ..oracle::PROP_ROOM };

pub static CARD: CardDef = CardDef { split: Some((&DAZZLING_THEATER, &PROP_ROOM)), ..oracle::DAZZLING_THEATER_PROP_ROOM };

pub static EXTRA: &[&CardDef] = &[&DAZZLING_THEATER, &PROP_ROOM];
