//! Dethrone (CR 702.105a), a triggered ability: "Whenever this creature attacks the player with the most life or tied for
//! most life, put a +1/+1 counter on this creature." Each instance triggers separately (CR 702.105b). Enraged Revolutionary,
//! Grenzo's Cutthroat and Marchesa's Emissary print it.
//!
//! A card with dethrone lists `TRIGGER` among its `abilities`. Its condition is
//! `triggers::this_attacks_the_player_with_the_most_life`, which Preacher of the Schism's first ability shares.

use crate::effects::plus_one_counter_on_this;
use crate::triggers::this_attacks_the_player_with_the_most_life;
use cantrip_api::*;

pub const TRIGGER: AbilityDef = triggered(
    when_asked(EventMask::ATTACKS, this_attacks_the_player_with_the_most_life),
    plus_one_counter_on_this,
    "Dethrone (Whenever this creature attacks the player with the most life or tied for most life, put a +1/+1 counter on it.)",
);
