use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::lands;
use cantrip_mechanics::tokens::{self, investigate};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

fn clue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::LANDFALL, clue, "Landfall \u{2014} Whenever a land you control enters, investigate."),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::you_sacrifice_a_subtype::<{ lands::sub(Subtype::Clue) }>),
            plus_one_counter_on_this,
            "Whenever you sacrifice a Clue, put a +1/+1 counter on Tireless Tracker.",
        )
        .on_sacrifice(),
    ],
    ..oracle::TIRELESS_TRACKER
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
