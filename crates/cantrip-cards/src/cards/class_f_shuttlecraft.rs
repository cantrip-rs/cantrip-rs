use cantrip_api::*;
use cantrip_mechanics::{statics, station};
use cantrip_oracle as oracle;

const SIX_PLUS: &str = "6+ | Flying, vigilance";

fn have_flying_and_vigilance(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Flying).with(Keyword::Vigilance);
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
        station::ability("Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)")
    ],
    statics: &[
        static_ability(Layer::Type, station::charged::<6>, station::is_a_creature, SIX_PLUS),
        static_ability(Layer::Ability, station::charged::<6>, have_flying_and_vigilance, SIX_PLUS).continues(),
        static_ability(Layer::PtSet, station::charged::<6>, statics::base_pt::<2, 4>, SIX_PLUS).continues(),
    ],
    ..oracle::CLASS_F_SHUTTLECRAFT
};
