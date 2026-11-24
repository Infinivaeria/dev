fn update(input) {
    if input.pitch_hz != () && input.pitch_hz > 300.0 {
        #{ band: "high", mirror: false }
    } else {
        #{ band: "auto" }
    }
}

