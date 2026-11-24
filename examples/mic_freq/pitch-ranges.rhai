fn update(input) {
    if input.pitch_hz == () { return #{ band: "auto" }; }
    if input.pitch_hz < 120.0 {
        #{ band: "low" }
    } else if input.pitch_hz < 260.0 {
        #{ band: "mid" }
    } else {
        #{ band: "high" }
    }
}

