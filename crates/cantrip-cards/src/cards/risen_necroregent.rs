use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::ZOMBIE];

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    delayed: &[triggered(YOUR_END_STEP, tokens::create_a_zombie, "At the beginning of your end step, create a 2/2 black Zombie creature token.")],
    statics: &[max_speed("Max speed \u{2014} At the beginning of your end step, create a 2/2 black Zombie creature token.")],
    ..oracle::RISEN_NECROREGENT
};
