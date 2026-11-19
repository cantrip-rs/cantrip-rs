use cantrip_api::*;
use cantrip_mechanics::statics::{have_trample, itself_with_permanent_cards_in_your_graveyard, plus_pt};
use cantrip_oracle as oracle;

/// "... can't be blocked by more than one creature" (CR 509.1b, layer 6) as a static ability's `StaticDef::apply`.
fn have_cant_be_blocked_by_more_than_one_creature(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::CantBeBlockedByMoreThanOne);
}

/// "Descend 4 - As long as there are four or more permanent cards in your graveyard, Akawalli gets +2/+2 and has trample" and
/// "Descend 8 - As long as there are eight or more permanent cards in your graveyard, Akawalli gets an additional +2/+2 and can't be
/// blocked by more than one creature" (CR 207.2c: descend is an ability word; 604.1, 611.3a, 613.4c, 613.6): each is one ability with
/// an effect in layer 6 and one in layer 7c, read from its controller's graveyard whenever characteristics are computed; at eight
/// cards both abilities apply and the bonuses add up to +4/+4. The layer-7c part continues the layer-6 one (`continues`), applying to
/// what that part applied to, so each ability's condition is named once and given to both parts.
const DESCEND_4: StaticAffects = itself_with_permanent_cards_in_your_graveyard::<4>;
const DESCEND_8: StaticAffects = itself_with_permanent_cards_in_your_graveyard::<8>;
const DESCEND_4_TEXT: &str = "Descend 4 \u{2014} As long as there are four or more permanent cards in your graveyard, Akawalli gets +2/+2 and has trample.";
const DESCEND_8_TEXT: &str =
    "Descend 8 \u{2014} As long as there are eight or more permanent cards in your graveyard, Akawalli gets an additional +2/+2 and can't be blocked by more than one creature.";

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, DESCEND_4, have_trample, DESCEND_4_TEXT),
        static_ability(Layer::PtModify, DESCEND_4, plus_pt::<2, 2>, DESCEND_4_TEXT).continues(),
        static_ability(Layer::Ability, DESCEND_8, have_cant_be_blocked_by_more_than_one_creature, DESCEND_8_TEXT),
        static_ability(Layer::PtModify, DESCEND_8, plus_pt::<2, 2>, DESCEND_8_TEXT).continues(),
    ],
    ..oracle::AKAWALLI_THE_SEETHING_TOWER
};
