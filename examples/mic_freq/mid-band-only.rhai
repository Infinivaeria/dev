fn update(input) {
    if input.pitch_hz != () && input.pitch_hz >= 160.0 && input.pitch_hz <= 300.0 {
        #{ band: "mid", mirror: false }
    } else {
        #{ band: "auto" }
    }
}

