#[path = "support/discard_library.rs"]
mod discard_library;
use discard_library::play;

/// Amnesia reveals the whole hand, so a discarded Probe Bones remains known when Leng puts it into the library.
#[test]
fn amnesia_keeps_discarded_cards_revealed_under_leng() {
    play(
        "amnesia-revealed-discard-under-leng",
        "
[Bot1 Deck]
Probe Leng
Black Lotus
Mox Sapphire
Mox Ruby
Mox Pearl
Amnesia
Island x20

[Bot2 Deck]
Probe Bones
Island x20

[Actions]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Amnesia.Cast(Bot2)
Bot1.Pass
Bot2.Pass
? Stack.Count = 1
Bot1.Pass
Bot2.Pass
[Bot1.Main2]

[Expect]
Bot2.Life = 23
Bot2.Graveyard.Count = 0
",
    );
}
