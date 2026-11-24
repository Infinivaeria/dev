fn update(input) {
    if input.pitch_hz == () { return #{ band: "auto" }; }
    let band = if input.pitch_hz < 90.0 { "low" }
        else if input.pitch_hz > 500.0 { "high" }
        else { "mid" };
    #{ band: band }
}

