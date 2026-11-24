fn update(input) {
    if input.adjusted_db > input.gate_db + input.loud_delta_db + 8.0 {
        #{ pose: "loud", bounce: 2.0, scale: 1.0 }
    } else {
        #{ pose: "auto" }
    }
}

