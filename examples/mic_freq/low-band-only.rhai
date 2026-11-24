fn update(input) {
    if input.pitch_hz != () && input.pitch_hz < 160.0 {
        #{ band: "low", mirror: false }
    } else {
        #{ band: "auto" }
    }
}

